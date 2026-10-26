use horizon_client::{
    Result,
    items::{ItemStack, Items},
};
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

pub fn main() -> Result<()> {
    let args: Vec<_> = crate::args().collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/horizon-client/26.3");
    let items = Items::catalog(&root)?;
    let mut out = BufWriter::new(File::create(&args[2])?);
    for line in BufReader::new(File::open(&args[1])?).lines() {
        let row: Value = serde_json::from_str(&line?)?;
        let mut stack = ItemStack::new(row["item"].as_str().unwrap(), 1);
        if let Some(tool) = row["tool"].as_str() {
            stack.data_mut().set_other(
                "minecraft:tool",
                Some(horizon_world::nbt::parse_snbt(tool)?.0),
            );
        }
        let block = row["block"].as_str().unwrap();
        writeln!(
            out,
            "{}",
            json!([
                items.mining_speed(Some(&stack), block).to_bits(),
                items.correct_for_drops(Some(&stack), block)
            ])
        )?;
    }
    Ok(())
}
