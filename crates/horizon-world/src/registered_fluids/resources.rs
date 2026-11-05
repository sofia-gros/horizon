//! Validated resource declarations shared by simulation, bucket and rendering
//! adapters. Absence leaves vanilla unchanged; malformed declarations fail load.
use super::{LiquidFamily, RegisteredFluidId, RegisteredFluidState, RegisteredFluids, valid_name};
use crate::{
    compression::invalid,
    registered_flow::{FlowPolicy, RegisteredFlow},
    registry::{BlockRegistry, BlockStateId},
};
use serde::Deserialize;
use std::{collections::HashMap, fs::File, io, path::Path, sync::Arc};

pub const RESOURCE_PATH: &str = "horizon/registered_fluids.json";
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FluidVisual {
    pub still: String,
    pub flowing: String,
    pub tint: u32,
    pub emission: u8,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    source: String,
    flowing: String,
    block: String,
    bucket: String,
    policy: FlowPolicy,
    visual: FluidVisual,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format: u32,
    fluids: Vec<Definition>,
}

/// Merge one mod-owned declaration into the aggregate resource document.
/// Full registry and policy validation follows after content block tables exist.
pub fn append_declaration(
    document: &mut serde_json::Value,
    owner: &str,
    bytes: &[u8],
) -> io::Result<()> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
    let declaration: Definition =
        serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string()))?;
    for name in [
        &declaration.source,
        &declaration.flowing,
        &declaration.block,
        &declaration.bucket,
    ] {
        if !valid_name(name)
            || name
                .split_once(':')
                .is_none_or(|(namespace, _)| namespace != owner)
        {
            return Err(invalid(
                "registered fluid identifiers must belong to the declaring mod",
            ));
        }
    }
    if document.get("format").and_then(serde_json::Value::as_u64) != Some(1) {
        return Err(invalid("unsupported registered-fluid aggregate format"));
    }
    let fluids = document
        .get_mut("fluids")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| invalid("missing aggregate fluid array"))?;
    for existing in fluids.iter() {
        for key in ["source", "flowing"] {
            let id = existing.get(key).and_then(serde_json::Value::as_str);
            if id == Some(declaration.source.as_str()) || id == Some(declaration.flowing.as_str()) {
                return Err(invalid("duplicate registered fluid identity"));
            }
        }
    }
    fluids.push(value);
    Ok(())
}

pub struct RegisteredFluidResources<'r> {
    pub catalog: Arc<RegisteredFluids<'r>>,
    pub flow: RegisteredFlow<'r, 'r>,
    buckets: HashMap<String, RegisteredFluidId>,
    source_buckets: HashMap<RegisteredFluidId, String>,
    visuals: HashMap<RegisteredFluidId, FluidVisual>,
}
impl<'r> RegisteredFluidResources<'r> {
    pub fn from_reader(registry: &'r BlockRegistry, reader: impl io::Read) -> io::Result<Self> {
        let document: Document =
            serde_json::from_reader(reader).map_err(|e| invalid(e.to_string()))?;
        if document.format != 1 {
            return Err(invalid("unsupported registered-fluid resource format"));
        }
        // Never run two simulators over vanilla liquid blocks or steal native
        // bucket use. Native-container integration needs a separate explicit path.
        if document.fluids.iter().any(|definition| {
            crate::fluid::FluidKind::from_name(&definition.source).is_some()
                || crate::fluid::FluidKind::from_name(&definition.flowing).is_some()
                || matches!(
                    definition.block.as_str(),
                    "minecraft:water" | "minecraft:lava"
                )
                || definition.bucket.starts_with("minecraft:")
        }) {
            return Err(invalid(
                "registered fluid declaration conflicts with vanilla liquid behavior",
            ));
        }
        let catalog = Arc::new(RegisteredFluids::new(
            registry,
            document.fluids.iter().map(|definition| LiquidFamily {
                source: definition.source.clone(),
                flowing: definition.flowing.clone(),
                block: definition.block.clone(),
            }),
        )?);
        let mut policies = Vec::new();
        let mut buckets = HashMap::new();
        let mut source_buckets = HashMap::new();
        let mut visuals = HashMap::new();
        for definition in document.fluids {
            if !valid_name(&definition.bucket)
                || !valid_name(&definition.visual.still)
                || !valid_name(&definition.visual.flowing)
                || definition.visual.tint > 0x00ff_ffff
                || definition.visual.emission > 15
            {
                return Err(invalid("invalid registered-fluid bucket or visual"));
            }
            let id = catalog
                .id(&definition.source)
                .ok_or_else(|| invalid("missing registered source"))?;
            if buckets.insert(definition.bucket.clone(), id).is_some() {
                return Err(invalid("bucket bound to multiple fluid families"));
            }
            source_buckets.insert(id, definition.bucket);
            visuals.insert(id, definition.visual);
            policies.push((id, definition.policy));
        }
        let flow = RegisteredFlow::new_shared(catalog.clone(), policies)?;
        Ok(Self {
            catalog,
            flow,
            buckets,
            source_buckets,
            visuals,
        })
    }
    pub fn load(registry: &'r BlockRegistry, root: &Path) -> io::Result<Option<Self>> {
        match File::open(root.join(RESOURCE_PATH)) {
            Ok(file) => Self::from_reader(registry, io::BufReader::new(file)).map(Some),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
    /// Validate item registration before making any bucket operation available.
    pub fn validate_buckets(&self, exists: impl Fn(&str) -> bool) -> io::Result<()> {
        if self.buckets.keys().any(|id| !exists(id)) {
            return Err(invalid("registered fluid bucket item is missing"));
        }
        Ok(())
    }
    pub fn bucket_fluid(&self, item: &str) -> Option<RegisteredFluidId> {
        self.buckets.get(item).copied()
    }
    pub fn bucket(&self, fluid: RegisteredFluidId) -> io::Result<&str> {
        self.source_buckets
            .get(&self.catalog.source(fluid)?)
            .map(String::as_str)
            .ok_or_else(|| invalid("registered fluid bucket is missing"))
    }
    pub fn visual(&self, state: BlockStateId) -> Option<(&FluidVisual, RegisteredFluidState)> {
        let fluid = self.catalog.from_block(state)?;
        Some((
            self.visuals.get(&self.catalog.source(fluid.id()).ok()?)?,
            fluid,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn registry() -> BlockRegistry {
        let rows:Vec<_>=(0..16).map(|level|json!(["test:liquid",false,true,"",{"id":"test:liquid","properties":{"level":level.to_string()}}])).collect();
        BlockRegistry::from_facts_json(
            json!({"states":rows,"defaults":{"test:liquid":0}})
                .to_string()
                .as_bytes(),
        )
        .unwrap()
    }
    fn document() -> serde_json::Value {
        json!({"format":1,"fluids":[{"source":"test:liquid","flowing":"test:flowing_liquid","block":"test:liquid","bucket":"test:liquid_bucket","policy":{"drop_off":3,"slope_distance":3,"tick_delay":30,"source_conversion":false},"visual":{"still":"test:block/liquid_still","flowing":"test:block/liquid_flow","tint":16777215,"emission":12}}]})
    }
    #[test]
    fn content_merge_rejects_foreign_or_duplicate_ids_without_mutating_aggregate() {
        let definition = document()["fluids"][0].clone();
        let mut aggregate = serde_json::json!({"format":1,"fluids":[]});
        append_declaration(&mut aggregate, "test", definition.to_string().as_bytes()).unwrap();
        let before = aggregate.clone();
        assert!(
            append_declaration(&mut aggregate, "test", definition.to_string().as_bytes()).is_err()
        );
        assert_eq!(aggregate, before);
        assert!(
            append_declaration(&mut aggregate, "other", definition.to_string().as_bytes()).is_err()
        );
        assert_eq!(aggregate, before);
        RegisteredFluidResources::from_reader(&registry(), aggregate.to_string().as_bytes())
            .unwrap();
    }
    #[test]
    fn declarations_bind_same_identity_for_flow_visuals_and_buckets() {
        let registry = registry();
        let resources =
            RegisteredFluidResources::from_reader(&registry, document().to_string().as_bytes())
                .unwrap();
        let id = resources.bucket_fluid("test:liquid_bucket").unwrap();
        let (visual, state) = resources.visual(BlockStateId(8)).unwrap();
        assert_eq!(resources.catalog.source(state.id()).unwrap(), id);
        assert!(state.falling());
        assert_eq!(visual.emission, 12);
        assert_eq!(resources.bucket(state.id()).unwrap(), "test:liquid_bucket");
        assert!(
            resources
                .validate_buckets(|id| id == "test:liquid_bucket")
                .is_ok()
        );
        assert!(resources.validate_buckets(|_| false).is_err());
        let scheduler = super::super::scheduling::RegisteredFluidScheduler::new_shared(
            resources.catalog.clone(),
        );
        drop(resources);
        assert_eq!(scheduler.count(), 0);
    }
    #[test]
    fn resource_registration_cannot_alias_native_fluid_or_bucket_behavior() {
        let registry = registry();
        for (key, value) in [
            ("source", "minecraft:water"),
            ("flowing", "minecraft:flowing_lava"),
            ("bucket", "minecraft:water_bucket"),
            ("block", "minecraft:water"),
        ] {
            let mut config = document();
            config["fluids"][0][key] = json!(value);
            assert!(
                RegisteredFluidResources::from_reader(&registry, config.to_string().as_bytes())
                    .is_err()
            );
        }
    }
    #[test]
    fn invalid_or_incomplete_resources_fail_instead_of_guessing_defaults() {
        let registry = registry();
        for (key, value) in [
            ("bucket", json!("unqualified")),
            (
                "visual",
                json!({"still":"test:a","flowing":"test:b","tint":16777216,"emission":0}),
            ),
            (
                "policy",
                json!({"drop_off":0,"slope_distance":3,"tick_delay":30,"source_conversion":false}),
            ),
        ] {
            let mut value_document = document();
            value_document["fluids"][0][key] = value;
            assert!(
                RegisteredFluidResources::from_reader(
                    &registry,
                    value_document.to_string().as_bytes()
                )
                .is_err()
            );
        }
        let mut missing = document();
        missing["fluids"][0]["policy"]
            .as_object_mut()
            .unwrap()
            .remove("source_conversion");
        assert!(
            RegisteredFluidResources::from_reader(&registry, missing.to_string().as_bytes())
                .is_err()
        );
        let mut version = document();
        version["format"] = json!(2);
        assert!(
            RegisteredFluidResources::from_reader(&registry, version.to_string().as_bytes())
                .is_err()
        );
    }
}
