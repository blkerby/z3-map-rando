use crate::{Patcher, PcAddr, SnesAddr, import::Importer};
use anyhow::{Result, anyhow};
use common_types::Item;
use patch_catalog::PatchCatalog;
use seed_data::ItemPlacement;

/// Add saved item writes using the current catalog's encodings and carrier hooks.
pub(crate) fn apply_items(
    patcher: &mut Patcher,
    rom: &[u8],
    placements: &[ItemPlacement],
    catalog: &PatchCatalog,
) -> Result<()> {
    patcher.use_ips(&catalog.patches.capacity_upgrades)?;
    patcher.use_ips(&catalog.patches.item_inventory)?;
    patcher.use_ips(&catalog.patches.item_menu_toggle)?;
    patcher.use_ips(&catalog.patches.item_placements)?;
    let tables = catalog.symbols.item_receipt_tables;
    let bank = tables & 0xFF0000;
    let mut context = patcher.context("vanilla item receipt tables");
    for (source, destination, length) in [
        (0x898366, tables, 0x4C),
        (0x8983B2, tables + 0x100, 0x4C),
        (0x8983FE, tables + 0x200, 0x4C),
        (0x89844A, tables + 0x300, 0x4C),
        (0x898496, tables + 0x400, 0x4C),
        (0x8984E2, tables + 0x500, 0x98),
        (0x89857A, tables + 0x700, 0x4C),
        (0x88C301, tables + 0x800, 0x98),
        (0x8985C6, bank | 0x85C6, 0x1C),
        (0x89891E, bank | 0x891E, 0x0C),
    ] {
        let start = PcAddr::from(SnesAddr(source)).0 as usize;
        let mut bytes = rom[start..start + length].to_vec();
        if destination == tables + 0x400 {
            bytes[1] = 0x05; // Use normal sword OAM properties for the Master Sword.
        }
        context.write(SnesAddr(destination).into(), bytes)?;
    }
    let graphics = Importer::new(rom.to_vec())?.read_item_graphics()?;
    patcher
        .context("vanilla item graphics")
        .write(SnesAddr(catalog.symbols.item_graphics).into(), graphics)?;
    for placement in placements {
        let location = &catalog.item_locations[&placement.item_location_id];
        if location.rom_addresses.is_empty() {
            continue;
        }
        let item = match placement.item {
            Item::SmallKey { .. } => &catalog.small_key,
            item => catalog
                .items
                .get(&item)
                .ok_or_else(|| anyhow!("no encoding for item {item:?}"))?,
        };
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
