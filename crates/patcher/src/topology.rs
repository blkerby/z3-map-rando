//! Relocate coordinate tables while keeping runtime area identities unchanged.

use crate::{Patcher, PcAddr, SnesAddr};
use anyhow::Result;
use patch_catalog::PatchCatalog;
use seed_data::WorldLayout;

pub(crate) fn apply_topology(
    patcher: &mut Patcher,
    rom: &[u8],
    layout: &WorldLayout,
    patches: &PatchCatalog,
) -> Result<()> {
    let read_word = |address: u32| {
        let pc: PcAddr = SnesAddr(address).into();
        u16::from_le_bytes(rom[pc.0 as usize..pc.0 as usize + 2].try_into().unwrap())
    };
    let owner_pc: PcAddr = SnesAddr(0x82a4e3).into();
    let vanilla_owners: [u8; 64] = rom[owner_pc.0 as usize..owner_pc.0 as usize + 64]
        .try_into()
        .unwrap();
    let mut owners = vanilla_owners;
    let mut anchors = std::array::from_fn::<_, 64, _>(|index| index as u8);
    let mut source_cells = std::array::from_fn::<_, 64, _>(|index| index as u8);
    for room in &layout.rooms {
        anchors[usize::from(room.vanilla_map_id)] = room.position;
        for dy in 0..room.size[1] {
            for dx in 0..room.size[0] {
                let cell = usize::from(room.position + dx + dy * 8);
                owners[cell] = room.vanilla_map_id;
                source_cells[cell] = room.vanilla_map_id + dx + dy * 8;
            }
        }
    }
    let mut context = patcher.context("overworld topology");
    context.write(
        SnesAddr(patches.symbols.slot_to_area).into(),
        owners.to_vec(),
    )?;
    context.write(
        SnesAddr(patches.symbols.area_to_anchor).into(),
        anchors.to_vec(),
    )?;
    if layout.rooms.is_empty() {
        return Ok(());
    }
    for (address, axis, bias) in [
        (0x82a7bb, 1, 0u16),
        (0x82a83b, 0, 0),
        (0x82be1a, 1, 0xff20),
        (0x82be9a, 0, 0xff00),
    ] {
        let mut bytes = Vec::new();
        for &owner in &owners {
            let anchor = anchors[usize::from(owner)];
            let coordinate = if axis == 0 { anchor % 8 } else { anchor / 8 };
            bytes.extend_from_slice(
                &(u16::from(coordinate) * 512)
                    .wrapping_add(bias)
                    .to_le_bytes(),
            );
        }
        context.write(SnesAddr(address).into(), bytes)?;
    }
    let mut tilemap_changes = Vec::new();
    for direction in 0..4 {
        for &cell in &source_cells {
            tilemap_changes.extend_from_slice(
                &read_word(0x82a52b + direction * 128 + u32::from(cell) * 2).to_le_bytes(),
            );
        }
    }
    context.write(SnesAddr(0x82a52b).into(), tilemap_changes)?;

    // Ordinary interior exits retain area IDs and area-local door offsets.
    for index in 0..79u32 {
        let pc: PcAddr = SnesAddr(0x82db8c + index).into();
        let area = rom[pc.0 as usize];
        if area >= 0x80 {
            continue;
        }
        let source = area & 0x3f;
        let anchor = anchors[usize::from(source)];
        let dx = (i16::from(anchor % 8) - i16::from(source % 8)) * 512;
        let dy = (i16::from(anchor / 8) - i16::from(source / 8)) * 512;
        for (address, delta) in [
            (0x82dc79, dy),
            (0x82dd17, dx),
            (0x82ddb5, dy),
            (0x82de53, dx),
            (0x82def1, dy),
            (0x82df8f, dx),
        ] {
            let address = address + index * 2;
            context.write(
                SnesAddr(address).into(),
                read_word(address)
                    .wrapping_add_signed(delta)
                    .to_le_bytes()
                    .to_vec(),
            )?;
        }
    }
    // Flute, Agahnim's warp, and whirlpool destination coordinates.
    for index in 0..17u32 {
        let source = read_word(0x82e849 + index * 2) as u8 & 0x3f;
        let anchor = anchors[usize::from(source)];
        let dx = (i16::from(anchor % 8) - i16::from(source % 8)) * 512;
        let dy = (i16::from(anchor / 8) - i16::from(source / 8)) * 512;
        for (address, delta) in [
            (0x82e88d, dy),
            (0x82e8af, dx),
            (0x82e8d1, dy),
            (0x82e8f3, dx),
            (0x82e915, dy),
            (0x82e937, dx),
        ] {
            let address = address + index * 2;
            context.write(
                SnesAddr(address).into(),
                read_word(address)
                    .wrapping_add_signed(delta)
                    .to_le_bytes()
                    .to_vec(),
            )?;
        }
    }

    // Gravestone activation and animation positions follow the cemetery.
    let anchor = anchors[0x14];
    let dx = (i16::from(anchor % 8) - 4) * 512;
    let dy = (i16::from(anchor / 8) - 2) * 512;
    for (address, count, delta) in [
        (0x899926, 8, dy),
        (0x899936, 15, dx),
        (0x899954, 15, dy),
        (0x899972, 15, dx),
    ] {
        for index in 0..count {
            let address = address + index * 2;
            context.write(
                SnesAddr(address).into(),
                read_word(address)
                    .wrapping_add_signed(delta)
                    .to_le_bytes()
                    .to_vec(),
            )?;
        }
    }

    // Outdoor trigger positions and the seven sets of Mode 7 objective markers.
    // The first waterfall trigger is indoors and the last follower trigger is
    // inside the special Master Sword grove; their coordinate spaces stay fixed.
    let mut position_tables = vec![
        (0x88f840, 0x88f838, 3, 2),
        (0x89a542, 0x89a540, 4, 10),
        (0x898c92, 0x898c8d, 1, 2),
        (0x898db3, 0x898dae, 1, 2),
        (0x9ee30a, 0x9ee316, 1, 2),
        (0x9ee576, 0x9ee570, 3, 2),
    ];
    for marker in 0..7 {
        position_tables.push((0x8abdf6 + marker * 0x24, 0x8abe08 + marker * 0x24, 9, 2));
    }
    for (x_address, y_address, count, stride) in position_tables {
        for index in 0..count {
            let x_address = x_address + index * stride;
            let y_address = y_address + index * stride;
            let x = read_word(x_address);
            let y = read_word(y_address);
            if x >= 0x1000 || y >= 0x1000 {
                continue;
            }
            let source = vanilla_owners[usize::from(y / 512 * 8 + x / 512)];
            let anchor = anchors[usize::from(source)];
            let dx = (i16::from(anchor % 8) - i16::from(source % 8)) * 512;
            let dy = (i16::from(anchor / 8) - i16::from(source / 8)) * 512;
            context.write(
                SnesAddr(x_address).into(),
                x.wrapping_add_signed(dx).to_le_bytes().to_vec(),
            )?;
            context.write(
                SnesAddr(y_address).into(),
                y.wrapping_add_signed(dy).to_le_bytes().to_vec(),
            )?;
        }
    }
    // The flute menu stores its eight marker coordinates as separate byte arrays.
    for index in 0..8 {
        let source = read_word(0x82e849 + index * 2) as u8 & 0x3f;
        let anchor = anchors[usize::from(source)];
        let dx = (i16::from(anchor % 8) - i16::from(source % 8)) * 512;
        let dy = (i16::from(anchor / 8) - i16::from(source / 8)) * 512;
        for (low_address, high_address, delta) in
            [(0x8ab783, 0x8ab78b, dx), (0x8ab793, 0x8ab79b, dy)]
        {
            let low: PcAddr = SnesAddr(low_address + index).into();
            let high: PcAddr = SnesAddr(high_address + index).into();
            let coordinate = u16::from_le_bytes([rom[low.0 as usize], rom[high.0 as usize]])
                .wrapping_add_signed(delta)
                .to_le_bytes();
            context.write(SnesAddr(low_address + index).into(), vec![coordinate[0]])?;
            context.write(SnesAddr(high_address + index).into(), vec![coordinate[1]])?;
        }
    }
    // Kakariko's flute-activation bounds and exploding-vane fragment origins.
    let anchor = anchors[0x18];
    let dx = i16::from(anchor % 8) * 512;
    let dy = (i16::from(anchor / 8) - 3) * 512;
    for (address, delta) in [
        (0x87a41c, dy),
        (0x87a421, dy),
        (0x87a428, dx),
        (0x87a42d, dx),
    ] {
        context.write(
            SnesAddr(address).into(),
            read_word(address)
                .wrapping_add_signed(delta)
                .to_le_bytes()
                .to_vec(),
        )?;
    }
    // All bat waypoints follow the Pyramid, including those outside its bounds.
    let anchor = anchors[0x1b];
    let dx = (i16::from(anchor % 8) - 3) * 512;
    let dy = (i16::from(anchor / 8) - 3) * 512;
    for (address, delta) in [(0x9af590, dx), (0x9af598, dy)] {
        for index in 0..4 {
            let address = address + index * 2;
            context.write(
                SnesAddr(address).into(),
                read_word(address)
                    .wrapping_add_signed(delta)
                    .to_le_bytes()
                    .to_vec(),
            )?;
        }
    }
    // Separate high-byte immediates for the vane, Kiki, and Pyramid bat cutscene.
    for (address, source, vertical) in [
        (0x898d51, 0x18u8, true),
        (0x898d5e, 0x18, false),
        (0x9ee34e, 0x1e, false),
        (0x9ee356, 0x1e, true),
        (0x9ee435, 0x1e, false),
        (0x9ee43d, 0x1e, true),
        (0x9af69b, 0x1b, false),
        (0x9af6a5, 0x1b, true),
        (0x9af71c, 0x1b, false),
        (0x9af726, 0x1b, true),
    ] {
        let anchor = anchors[usize::from(source)];
        let delta = if vertical {
            (i16::from(anchor / 8) - i16::from(source / 8)) * 2
        } else {
            (i16::from(anchor % 8) - i16::from(source % 8)) * 2
        };
        let pc: PcAddr = SnesAddr(address).into();
        context.write(
            pc,
            vec![rom[pc.0 as usize].wrapping_add_signed(delta as i8)],
        )?;
    }
    // Castle/Pyramid's legacy camera clamp and the digging-game field boundary.
    for (address, source) in [(0x82bb87, 0x1b), (0x82bb90, 0x1b), (0x9dfd61, 0x28)] {
        let dy = (i16::from(anchors[source] / 8) - source as i16 / 8) * 512;
        context.write(
            SnesAddr(address).into(),
            read_word(address)
                .wrapping_add_signed(dy)
                .to_le_bytes()
                .to_vec(),
        )?;
    }
    Ok(())
}
