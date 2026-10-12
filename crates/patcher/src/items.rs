use crate::{Patcher, PcAddr};
use anyhow::Result;
use patch_catalog::PatchCatalog;
use seed_data::ItemPlacement;

/// Add saved item writes using the current catalog's encodings and carrier hooks.
pub(crate) fn apply_items(
    patcher: &mut Patcher,
    placements: &[ItemPlacement],
    catalog: &PatchCatalog,
) -> Result<()> {
    patcher.use_ips(&catalog.patches.item_placements)?;
    for placement in placements {
        let location = &catalog.item_locations[&placement.item_location_id];
        if location.rom_addresses.is_empty() {
            continue;
        }
        let item = &catalog.items[&placement.item_name];
        let mut context = patcher.context(&format!("item {:?}", placement.item_location_id));
        if let Some(bytes) = &item.prize_patch_bytes {
            for (&address, &value) in location.rom_addresses.iter().zip(bytes) {
                context.write(PcAddr(address), vec![value])?;
            }
        } else {
            context.write(PcAddr(location.rom_addresses[0]), vec![item.receipt_id])?;
        }
    }
    Ok(())
}
