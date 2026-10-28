//! Mod-owned, data-driven item appearances. A rule only reads custom data in
//! its own namespace and can only select a client item in that namespace.
use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    item: String,
    path: Vec<String>,
    cases: Vec<Case>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    when: BTreeMap<String, Value>,
    model: String,
}
#[derive(Debug)]
pub(super) struct Appearance {
    file: File,
}

pub(super) fn load(root: &Path, definitions: &BTreeMap<String, ClientItem>) -> Vec<Appearance> {
    let mut out = Vec::new();
    let Ok(namespaces) = fs::read_dir(root.join("assets")) else {
        return out;
    };
    for ns in namespaces.flatten() {
        let namespace = ns.file_name().to_string_lossy().into_owned();
        if namespace == "minecraft" {
            continue;
        }
        let Ok(files) = fs::read_dir(ns.path().join("horizon/item_appearances")) else {
            continue;
        };
        for entry in files.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let Ok(bytes) = fs::read(path) else { continue };
            if bytes.len() > 65536 {
                continue;
            }
            let Ok(file) = serde_json::from_slice::<File>(&bytes) else {
                continue;
            };
            if file.path.len() < 2
                || file.path.len() > 8
                || file.path[0] != namespace
                || file.path.iter().any(|p| p.is_empty() || p.len() > 128)
                || file.item.len() > 256
                || file.cases.is_empty()
                || file.cases.len() > 128
                || file.cases.iter().any(|c| {
                    c.when.is_empty()
                        || c.when.len() > 16
                        || c.when.keys().any(|p| !p.starts_with('/') || p.len() > 256)
                        || !c.model.starts_with(&format!("{namespace}:"))
                        || !definitions.contains_key(&c.model)
                })
            {
                continue;
            }
            out.push(Appearance { file });
        }
    }
    out
}

/// Ambiguous matches deliberately keep the original model, independent of load order.
pub(super) fn select<'a>(rules: &'a [Appearance], stack: &StackView) -> Option<&'a str> {
    let data = stack.patch.get("minecraft:custom_data")?.as_ref()?;
    let mut selected = None;
    for rule in rules.iter().filter(|r| r.file.item == stack.item) {
        let mut value = data;
        for key in &rule.file.path {
            let Some(next) = value.get(key) else {
                value = &Value::Null;
                break;
            };
            value = next;
        }
        let Some(json) = value.as_str().filter(|s| s.len() <= 4096) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<Value>(json) else {
            continue;
        };
        for case in &rule.file.cases {
            if case
                .when
                .iter()
                .all(|(pointer, expected)| json.pointer(pointer) == Some(expected))
            {
                if selected.is_some() {
                    return None;
                }
                selected = Some(case.model.as_str());
            }
        }
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rule() -> Appearance {
        Appearance { file: serde_json::from_value(json!({
            "item": "minecraft:wooden_pickaxe", "path": ["example", "state"],
            "cases": [
                {"when": {"/tool": "example:pick", "/materials/0": "wood", "/broken": false}, "model": "example:wood"},
                {"when": {"/tool": "example:pick", "/materials/0": "flint", "/broken": true}, "model": "example:flint_broken"}
            ]
        })).unwrap() }
    }
    fn stack(state: &str) -> StackView {
        let mut s = StackView::new("minecraft:wooden_pickaxe", 1);
        s.patch.insert(
            "minecraft:custom_data".into(),
            Some(json!({"example": {"state": state}})),
        );
        s
    }
    #[test]
    fn selects_from_each_stacks_current_state_and_falls_back() {
        let r = vec![rule()];
        let s = stack(r#"{"tool":"example:pick","materials":["wood"],"broken":false,"damage":5}"#);
        assert_eq!(select(&r, &s), Some("example:wood"));
        let s = stack(r#"{"tool":"example:pick","materials":["flint"],"broken":true}"#);
        assert_eq!(select(&r, &s), Some("example:flint_broken"));
        assert_eq!(select(&r, &stack("{")), None);
        assert_eq!(select(&r, &stack(&"x".repeat(4097))), None);
        assert_eq!(
            select(&r, &StackView::new("minecraft:wooden_pickaxe", 1)),
            None
        );
        let mut other = s.clone();
        other.item = "minecraft:stone_pickaxe".into();
        assert_eq!(select(&r, &other), None);
        assert_eq!(select(&[rule(), rule()], &s), None);
    }
    #[test]
    fn loading_rejects_cross_namespace_data_and_models() {
        let root = std::env::temp_dir().join(format!("horizon-appearance-{}", std::process::id()));
        let dir = root.join("assets/example/horizon/item_appearances");
        fs::create_dir_all(&dir).unwrap();
        let definition = ClientItem::from_json(r#"{"model":{"type":"minecraft:empty"}}"#).unwrap();
        let definitions = BTreeMap::from([
            ("example:wood".into(), definition.clone()),
            ("other:wood".into(), definition),
        ]);
        let valid = json!({"item":"minecraft:wooden_pickaxe","path":["example","state"],"cases":[{"when":{"/tool":"x"},"model":"example:wood"}]});
        fs::write(dir.join("valid.json"), valid.to_string()).unwrap();
        let mut foreign_path = valid.clone();
        foreign_path["path"][0] = json!("other");
        fs::write(dir.join("foreign_path.json"), foreign_path.to_string()).unwrap();
        let mut foreign_model = valid.clone();
        foreign_model["cases"][0]["model"] = json!("other:wood");
        fs::write(dir.join("foreign_model.json"), foreign_model.to_string()).unwrap();
        assert_eq!(load(&root, &definitions).len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
