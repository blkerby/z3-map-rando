use anyhow::{Context, Result};
use std::{collections::BTreeMap, io::Read, path::Path};

pub mod import;

#[derive(PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub struct PcAddr(pub u32);

#[derive(PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub struct SnesAddr(pub u32);

impl From<SnesAddr> for PcAddr {
    fn from(value: SnesAddr) -> Self {
        Self(((value.0 & 0x7f0000) >> 1) | (value.0 & 0x7fff))
    }
}

impl From<PcAddr> for SnesAddr {
    fn from(value: PcAddr) -> Self {
        Self(0x8000 | ((value.0 & 0x3f8000) << 1) | (value.0 & 0x7fff))
    }
}

type ContextId = u32;

#[derive(Default)]
pub struct Patcher {
    contexts: Vec<String>,
    patches: BTreeMap<PcAddr, (ContextId, Vec<u8>)>,
}

pub struct PatchContext<'a> {
    id: ContextId,
    rom: &'a mut Patcher,
}

impl Patcher {
    pub fn context(&mut self, ctx: &str) -> PatchContext<'_> {
        let id: ContextId = self
            .contexts
            .len()
            .try_into()
            .expect("Too many patch contexts");
        self.contexts.push(ctx.to_string());
        PatchContext { id, rom: self }
    }

    pub fn use_ips(&mut self, path: &Path) -> Result<()> {
        let mut input = std::fs::File::open(path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        let mut header = [0; 5];
        input.read_exact(&mut header)?;
        anyhow::ensure!(
            &header == b"PATCH",
            "invalid IPS header in {}",
            path.display()
        );

        let filename = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy();
        let mut context = self.context(&filename);
        loop {
            let mut offset = [0; 3];
            input.read_exact(&mut offset)?;
            if &offset == b"EOF" {
                return Ok(());
            }

            let addr = PcAddr(u32::from_be_bytes([0, offset[0], offset[1], offset[2]]));
            let mut size = [0; 2];
            input.read_exact(&mut size)?;
            let size = usize::from(u16::from_be_bytes(size));
            let buf = if size == 0 {
                let mut rle_size = [0; 2];
                let mut value = [0; 1];
                input.read_exact(&mut rle_size)?;
                input.read_exact(&mut value)?;
                vec![value[0]; usize::from(u16::from_be_bytes(rle_size))]
            } else {
                let mut buf = vec![0; size];
                input.read_exact(&mut buf)?;
                buf
            };
            context.write(addr, buf)?;
        }
    }

    pub fn apply(&self, rom: &mut [u8]) -> Result<()> {
        for (addr, (context_id, buf)) in &self.patches {
            let start = addr.0 as usize;
            let end = start + buf.len();
            let Some(output) = rom.get_mut(start..end) else {
                anyhow::bail!(
                    "{} writes past the end of the ROM",
                    self.contexts[*context_id as usize]
                );
            };
            output.copy_from_slice(buf);
        }
        Ok(())
    }
}

impl<'a> PatchContext<'a> {
    pub fn write(&mut self, addr: PcAddr, buf: Vec<u8>) -> Result<()> {
        let start = addr.0;
        let end = start + buf.len() as u32;
        let overlaps = |existing_addr: &PcAddr, existing_buf: &[u8]| {
            start < existing_addr.0 + existing_buf.len() as u32 && existing_addr.0 < end
        };

        // Ensure that this write does not conflict with any previous write.
        // Since patches are in sorted order by starting address, it is sufficient to
        // check overlap with either the previous or next patch.
        if let Some((existing_addr, (existing_id, existing_buf))) = self
            .rom
            .patches
            .range(..addr)
            .next_back()
            .filter(|(existing_addr, (_, existing_buf))| overlaps(existing_addr, existing_buf))
            .or_else(|| {
                self.rom.patches.range(addr..).next().filter(
                    |(existing_addr, (_, existing_buf))| overlaps(existing_addr, existing_buf),
                )
            })
        {
            anyhow::bail!(
                "patches overlap: {} (${addr:06X}, {} bytes) and {} (${new_addr:06X}, {} bytes)",
                self.rom.contexts[*existing_id as usize],
                existing_buf.len(),
                self.rom.contexts[self.id as usize],
                buf.len(),
                addr = SnesAddr::from(*existing_addr).0,
                new_addr = SnesAddr::from(addr).0,
            );
        }

        self.rom.patches.insert(addr, (self.id, buf));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_lorom_addresses() {
        assert_eq!(PcAddr::from(SnesAddr(0x008000)).0, 0x000000);
        assert_eq!(PcAddr::from(SnesAddr(0x00ffff)).0, 0x007fff);
        assert_eq!(PcAddr::from(SnesAddr(0x018000)).0, 0x008000);
        assert_eq!(PcAddr::from(SnesAddr(0x7fffff)).0, 0x3fffff);

        assert_eq!(SnesAddr::from(PcAddr(0x000000)).0, 0x008000);
        assert_eq!(SnesAddr::from(PcAddr(0x007fff)).0, 0x00ffff);
        assert_eq!(SnesAddr::from(PcAddr(0x008000)).0, 0x018000);
        assert_eq!(SnesAddr::from(PcAddr(0x3fffff)).0, 0x7fffff);

        assert_eq!(PcAddr::from(SnesAddr(0x008000)).0, 0x000000);
    }

    #[test]
    fn write_rejects_overlaps_and_accepts_adjacent_patches() {
        let mut patcher = Patcher {
            contexts: Vec::new(),
            patches: BTreeMap::new(),
        };

        patcher
            .context("existing")
            .write(PcAddr(10), vec![0; 4])
            .unwrap();
        patcher
            .context("adjacent")
            .write(PcAddr(14), vec![0])
            .unwrap();

        let error = patcher
            .context("new")
            .write(PcAddr(12), vec![0; 4])
            .unwrap_err()
            .to_string();

        assert_eq!(
            error,
            "patches overlap: existing ($00800A, 4 bytes) and new ($00800C, 4 bytes)"
        );

        let error = patcher
            .context("same start")
            .write(PcAddr(10), vec![0])
            .unwrap_err()
            .to_string();

        assert!(error.contains("existing"));
        assert!(error.contains("same start"));
        assert_eq!(patcher.patches.len(), 2);
    }

    #[test]
    fn applies_ips_records() {
        let path = std::env::temp_dir().join(format!("patcher-apply-{}.ips", std::process::id()));
        std::fs::write(
            &path,
            b"PATCH\x00\x00\x10\x00\x02\xaa\xbb\x00\x00\x12\x00\x00\x00\x03\xccEOF",
        )
        .unwrap();

        let mut patcher = Patcher {
            contexts: Vec::new(),
            patches: BTreeMap::new(),
        };

        let result = patcher.use_ips(&path);
        std::fs::remove_file(&path).unwrap();
        result.unwrap();

        assert_eq!(
            patcher.contexts[0],
            path.file_name().unwrap().to_string_lossy()
        );
        assert_eq!(patcher.patches[&PcAddr(0x10)].1, [0xaa, 0xbb]);
        assert_eq!(patcher.patches[&PcAddr(0x12)].1, [0xcc; 3]);

        let mut rom = vec![0; 0x15];
        patcher.apply(&mut rom).unwrap();
        assert_eq!(&rom[0x10..], &[0xaa, 0xbb, 0xcc, 0xcc, 0xcc]);
    }
}
