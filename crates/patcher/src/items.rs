use crate::{Patcher, PcAddr, SnesAddr, import::Importer};
use anyhow::{Result, anyhow};
use common_types::ItemLocationId;
use patch_catalog::PatchCatalog;
use seed_data::RoomData;

/// Add saved item writes using the current catalog's encodings and carrier hooks.
pub(crate) fn apply_items(
    patcher: &mut Patcher,
    rom: &[u8],
    rooms: &[RoomData],
    catalog: &PatchCatalog,
) -> Result<()> {
    patcher.use_ips(&catalog.patches.capacity_upgrades)?;
    patcher.use_ips(&catalog.patches.fairy_ponds)?;
    patcher.use_ips(&catalog.patches.item_inventory)?;
    patcher.use_ips(&catalog.patches.item_menu_toggle)?;
    patcher.use_ips(&catalog.patches.item_placements)?;
    patcher.use_ips(&catalog.patches.paired_keys)?;
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
        (0x89891E, bank | 0x891E, 0x07),
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
    let key_doors = &catalog.key_doors;
    let mut context = patcher.context("paired key doors");
    context.write(
        SnesAddr(catalog.symbols.paired_key_locks).into(),
        key_doors.locks.clone(),
    )?;
    context.write(
        SnesAddr(catalog.symbols.paired_key_messages).into(),
        key_doors.messages.clone(),
    )?;
    for room in rooms {
        for placement in &room.items {
            let id = ItemLocationId {
                room_id: room.room_id,
                item_id: placement.item_id,
            };
            let location = &catalog.item_locations[&id];
            if let common_types::Item::SmallKey(door) = placement.item {
                let index = key_doors
                    .indices
                    .get(&door)
                    .ok_or_else(|| anyhow!("unknown Small Key door {door:?}"))?;
                let address = location
                    .rom_addresses
                    .first()
                    .ok_or_else(|| anyhow!("unknown Small Key pickup {id:?}"))?;
                patcher
                    .context("paired key pickup")
                    .write(PcAddr(*address), vec![*index])?;
                continue;
            }
            if location.rom_addresses.is_empty() {
                continue;
            }
            let kind = placement.item.get_kind();
            let item = catalog
                .items
                .get(&kind)
                .ok_or_else(|| anyhow!("no encoding for item {kind:?}"))?;
            let mut context = patcher.context(&format!("item {id:?}"));
            if let Some(bytes) = &item.prize_patch_bytes {
                for (&address, &value) in location.rom_addresses.iter().zip(bytes) {
                    context.write(PcAddr(address), vec![value])?;
                }
            } else {
                context.write(PcAddr(location.rom_addresses[0]), vec![item.receipt_id])?;
            }
        }
    }
    Ok(())
}
