//! OMKF v1 Zero-Copy Binary Capsule (`64 B` Header + `928 B` Slots) & `.hera/crate.md` Codec.

use crate::scanner::audio_metrics::cosine_similarity_i8;
use crate::synergy::parse_camelot;
#[cfg(not(target_arch = "wasm32"))]
use std::{fs, io::Read, path::Path};

pub const OMKF_MAGIC: [u8; 8] = *b"OMKF\x01\x00\x00\x00";
pub const OMKF_HEADER_BYTES: usize = 64;
pub const OMKF_SLOT_BYTES: usize = 928;
pub const OMKF_FLAG_IS_VIDEO: u16 = 1 << 0;
pub const OMKF_FLAG_IS_TRANSITION: u16 = 1 << 1;
pub const OMKF_FLAG_IS_VARIABLE_BPM: u16 = 1 << 2;
pub const OMKF_FLAG_IS_FAKE_320KBPS: u16 = 1 << 3;
pub const OMKF_FLAG_HAS_AUDIO_SKIPS: u16 = 1 << 4;
pub const OMKF_FLAG_HAS_INTRO_VOCALS: u16 = 1 << 5;
pub const OMKF_FLAG_HAS_OUTRO_VOCALS: u16 = 1 << 6;
pub const OMKF_FLAG_IS_OWN_PRODUCTION: u16 = 1 << 7;
pub const OMKF_FLAG_IS_MIX_OVER_10M: u16 = 1 << 8;
pub const OMKF_FLAG_HAS_BPM_TRUST_DIFF: u16 = 1 << 9;

trait LePod: Sized + Copy {
    const SZ: usize;
    fn wr(&self, d: &mut [u8]);
    fn rd(s: &[u8]) -> Self;
}
macro_rules! impl_le_pods { ($($t:ty),*) => {
    $(impl LePod for $t { const SZ: usize = std::mem::size_of::<$t>(); fn wr(&self, d: &mut [u8]) { d[..Self::SZ].copy_from_slice(&self.to_le_bytes()); } fn rd(s: &[u8]) -> Self { Self::from_le_bytes(std::array::from_fn(|i| s[i])) } })*
    impl<const N: usize> LePod for [u8; N] { const SZ: usize = N; fn wr(&self, d: &mut [u8]) { d[..N].copy_from_slice(self); } fn rd(s: &[u8]) -> Self { std::array::from_fn(|i| s[i]) } }
    impl<const N: usize> LePod for [i8; N] { const SZ: usize = N; fn wr(&self, d: &mut [u8]) { d[..N].copy_from_slice(&self.map(|x| x as u8)); } fn rd(s: &[u8]) -> Self { std::array::from_fn(|i| s[i] as i8) } }
    impl LePod for [u32; 8] { const SZ: usize = 32; fn wr(&self, d: &mut [u8]) { for (i, v) in self.iter().enumerate() { v.wr(&mut d[i * 4..]); } } fn rd(s: &[u8]) -> Self { std::array::from_fn(|i| u32::rd(&s[i * 4..])) } }
}; }
impl_le_pods!(u8, i8, u16, i16, u32, u64, i64);

macro_rules! omkf_le_struct { ($name:ident, $sz:expr, $head:expr, $init:expr, $($f:ident : $t:ty),* $(,)?) => {
    #[derive(Debug, Clone, PartialEq, Eq)] pub struct $name { $(pub $f: $t),* }
    impl $name {
        pub fn to_bytes(&self) -> [u8; $sz] {
            let (mut b, mut o) = ([0_u8; $sz], $head);
            if $head == 8 { b[0..8].copy_from_slice(&OMKF_MAGIC); }
            $( self.$f.wr(&mut b[o..]); o += <$t as LePod>::SZ; )* let _ = o; b
        }
        pub fn from_bytes(b: &[u8]) -> Result<Self, String> {
            if b.len() < $sz || ($head == 8 && b[0..8] != OMKF_MAGIC) { return Err("Invalid OMKF".into()); }
            let mut o = $head; let s = Self { $($f: { let v = <$t as LePod>::rd(&b[o..]); o += <$t as LePod>::SZ; v }),* }; let _ = o; Ok(s)
        }
    }
    impl Default for $name { fn default() -> Self {
        let mut b = [0_u8; $sz]; if $head == 8 { b[0..8].copy_from_slice(&OMKF_MAGIC); }
        let mut s = Self::from_bytes(&b).expect("sz"); ($init)(&mut s); s
    } }
}; }
omkf_le_struct!(OmkfHeader, OMKF_HEADER_BYTES, 8, |h: &mut Self| h.strtab_offset = 64,
    tree_seal_64: u64, dir_mtime_ns: i64, track_count: u32, strtab_offset: u32,
    bpm_min_x100: u16, bpm_max_x100: u16, camelot_mask: u32, centroid_top24_i8: [i8; 24]);
omkf_le_struct!(OmkfTrackSlot, OMKF_SLOT_BYTES, 0,
    |s: &mut Self| (s.lufs_x100, s.sub_phase_x1000) = (-7000, 1000),
    partial_hash_64kb: [u8; 16], file_size_bytes: u64, mtime_ns: i64, bpm_x100: u16,
    entry_bpm_x100: u16, exit_bpm_x100: u16, lufs_x100: i16, replay_gain_x100: i16,
    camelot_code: u8, scan_level: i8, flags_bits: u16, duration_sec_u16: u16,
    hot_cues_ms: [u32; 8], name_offset: u32, name_len: u16, sample_rate_div100: u16,
    genre_family_code: u8, genre_confidence_pct: u8, inferred_era_year: u16,
    sub_phase_x1000: i16, bitrate_kbps: u16, embedding_64d: [i8; 64], rgb_peaks_256: [u8; 768]);
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OmkfTrackEntry {
    pub slot: OmkfTrackSlot,
    pub filename: String,
    pub artist: String,
    pub title: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OmkfCapsule {
    pub header: OmkfHeader,
    pub tracks: Vec<OmkfTrackEntry>,
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OmkfCrateMdMetadata {
    pub crate_id: String,
    pub tree_seal_64: u64,
    pub track_count: u32,
    pub video_count: u32,
    pub bpm_span: String,
    pub dominant_camelot: String,
    pub dominant_genre: String,
    pub era_span: String,
    pub lufs_avg: f32,
    pub health: String,
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OmkfCratePruneCandidate {
    pub crate_id: String,
    pub bpm_min: Option<f32>,
    pub bpm_max: Option<f32>,
    pub camelot_mask: u32,
    pub centroid_top24_i8: [i8; 24],
}

pub fn camelot_to_code(c: &str) -> u8 {
    parse_camelot(c).map_or(0, |(n, l)| n as u8 + if l == 'B' { 12 } else { 0 })
}
pub fn code_to_camelot(c: u8) -> Option<String> {
    let ab = if c <= 12 { 'A' } else { 'B' };
    (1..=24)
        .contains(&c)
        .then(|| format!("{}{ab}", ((c - 1) % 12) + 1))
}
pub fn camelot_code_mask_bit(c: u8) -> u32 {
    u32::from((1..=24).contains(&c)) << c.saturating_sub(1)
}
pub fn compute_entry_seal_u64(filename: &str, file_size: u64, mtime_ns: i64) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for b in filename.bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    h ^ file_size.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (mtime_ns as u64).rotate_left(13)
}
pub fn compute_tree_seal_u64(entry_seals: &[u64]) -> u64 {
    entry_seals.iter().copied().fold(0, |a, s| a ^ s)
}

pub fn encode_omkf_capsule(header: &OmkfHeader, tracks: &[OmkfTrackEntry]) -> Vec<u8> {
    let slots_len = tracks.len() * OMKF_SLOT_BYTES;
    let mut hdr = header.clone();
    hdr.track_count = tracks.len() as u32;
    hdr.strtab_offset = (OMKF_HEADER_BYTES + slots_len) as u32;
    let mut st = Vec::new();
    let mut out = Vec::with_capacity(64 + slots_len + tracks.len() * 48);
    out.extend_from_slice(&hdr.to_bytes());
    for e in tracks {
        let cl = |s: &str| s.replace('\0', " ");
        let mut rec = format!("{}\0{}\0{}", cl(&e.filename), cl(&e.artist), cl(&e.title));
        while rec.len() > u16::MAX as usize {
            rec.pop();
        }
        let mut s = e.slot.clone();
        (s.name_offset, s.name_len) = (st.len() as u32, rec.len() as u16);
        (s.genre_family_code, s.genre_confidence_pct) =
            (s.genre_family_code.min(14), s.genre_confidence_pct.min(100));
        st.extend_from_slice(rec.as_bytes());
        out.extend_from_slice(&s.to_bytes());
    }
    out.extend_from_slice(&st);
    out
}

pub fn decode_omkf_capsule(bytes: &[u8]) -> Result<OmkfCapsule, String> {
    let header = OmkfHeader::from_bytes(bytes)?;
    let n = header.track_count as usize;
    let max_n = bytes.len().saturating_sub(OMKF_HEADER_BYTES) / OMKF_SLOT_BYTES;
    let st_off = header.strtab_offset as usize;
    let end = OMKF_HEADER_BYTES + n.min(max_n) * OMKF_SLOT_BYTES;
    if n > max_n || st_off < end || st_off > bytes.len() {
        return Err("Bad strtab".into());
    }
    let st = &bytes[st_off..];
    let mut tracks = Vec::with_capacity(n);
    for i in 0..n {
        let slot = OmkfTrackSlot::from_bytes(&bytes[OMKF_HEADER_BYTES + i * OMKF_SLOT_BYTES..])?;
        let o = slot.name_offset as usize;
        let sl = st.get(o..o + slot.name_len as usize).ok_or("strtab OOB")?;
        let txt = std::str::from_utf8(sl).map_err(|_| "UTF-8")?;
        let mut p = txt.splitn(3, '\0');
        let mut f = || p.next().unwrap_or("").to_owned();
        let mut tr = OmkfTrackEntry::default();
        (tr.slot, tr.filename, tr.artist, tr.title) = (slot, f(), f(), f());
        tracks.push(tr);
    }
    Ok(OmkfCapsule { header, tracks })
}

pub fn merge_crate_md_frontmatter(existing_md: Option<&str>, m: &OmkfCrateMdMetadata) -> String {
    let def = "# Intención y Reglas de Cabina\n\n# Ledger Reversible\n";
    let body = existing_md
        .and_then(|s| s.trim_start().strip_prefix("---"))
        .and_then(|r| r.trim_start_matches(['\r', '\n']).split_once("\n---"))
        .map(|(_, t)| t.trim_start_matches(['\r', '\n']))
        .unwrap_or_else(|| existing_md.filter(|s| !s.trim().is_empty()).unwrap_or(def));
    format!("---\nomkf: 1\ncrate_id: \"{}\"\ntree_seal_64: \"{:016x}\"\ntrack_count: {}\nvideo_count: {}\nbpm_span: \"{}\"\ndominant_camelot: \"{}\"\ndominant_genre: \"{}\"\nera_span: \"{}\"\nlufs_avg: {:.2}\nhealth: \"{}\"\n---\n\n{body}", m.crate_id, m.tree_seal_64, m.track_count, m.video_count, m.bpm_span, m.dominant_camelot, m.dominant_genre, m.era_span, m.lufs_avg, m.health)
}

pub fn compute_centroid_top24_i8(slots: &[OmkfTrackSlot]) -> [i8; 24] {
    let mut acc = [0_i64; 24];
    slots
        .iter()
        .for_each(|s| (0..24).for_each(|i| acc[i] += i64::from(s.embedding_64d[i])));
    let d = slots.len().max(1) as i64;
    std::array::from_fn(|i| (acc[i] / d).clamp(-127, 127) as i8)
}

pub fn prune_crates_stage1(
    crates: &[OmkfCratePruneCandidate],
    q: &[i8; 64],
    bpm: Option<(f32, f32)>,
    mask: u32,
    k: usize,
) -> Vec<String> {
    let mut hits = Vec::new();
    for (idx, c) in crates.iter().enumerate() {
        let bpm_ok = !matches!((bpm, c.bpm_min, c.bpm_max), (Some((a, b)), Some(x), Some(y)) if y < a || x > b);
        if !bpm_ok || (mask != 0 && c.camelot_mask != 0 && (c.camelot_mask & mask) == 0) {
            continue;
        }
        let (mut d, mut na, mut nb) = (0_i32, 0_i32, 0_i32);
        for (&ca, &qb) in c.centroid_top24_i8.iter().zip(q) {
            let (a, b) = (i32::from(ca), i32::from(qb));
            (d, na, nb) = (d + a * b, na + a * a, nb + b * b);
        }
        let den = (na as f32 * nb as f32).sqrt();
        hits.push((idx, if den > 0.0 { d as f32 / den } else { 0.0 }));
    }
    hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(k);
    let to_id = |(i, _): &(usize, f32)| crates[*i].crate_id.clone();
    hits.iter().map(to_id).collect()
}

pub fn rank_slots_stage2(slots: &[OmkfTrackSlot], q: &[i8; 64], top_k: usize) -> Vec<(usize, f32)> {
    let cos = |s: &OmkfTrackSlot| cosine_similarity_i8(&s.embedding_64d, q);
    let mut hits: Vec<_> = slots.iter().enumerate().map(|(i, s)| (i, cos(s))).collect();
    hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(top_k);
    hits
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_omkf_header(p: &Path) -> Result<OmkfHeader, String> {
    let mut b = [0_u8; OMKF_HEADER_BYTES];
    fs::File::open(p)
        .and_then(|mut f| f.read_exact(&mut b))
        .map_err(|e| e.to_string())?;
    OmkfHeader::from_bytes(&b)
}
#[cfg(not(target_arch = "wasm32"))]
pub fn read_omkf_capsule(p: &Path) -> Result<OmkfCapsule, String> {
    decode_omkf_capsule(&fs::read(p).map_err(|e| e.to_string())?)
}
#[cfg(not(target_arch = "wasm32"))]
pub fn write_omkf_capsule_atomic(
    d: &Path,
    h: &OmkfHeader,
    tr: &[OmkfTrackEntry],
    md: &OmkfCrateMdMetadata,
) -> Result<(), String> {
    fs::create_dir_all(d).map_err(|e| e.to_string())?;
    let old = fs::read_to_string(d.join("crate.md")).ok();
    let b1 = encode_omkf_capsule(h, tr);
    let b2 = merge_crate_md_frontmatter(old.as_deref(), md).into_bytes();
    let pairs = [("tracks.omkf", b1), ("crate.md", b2)];
    for (name, bytes) in pairs {
        let (tmp, dst) = (d.join(format!("{name}.tmp")), d.join(name));
        fs::write(&tmp, &bytes)
            .and_then(|_| fs::rename(&tmp, dst))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_omkf_roundtrip_seal_md_and_prune() {
        assert_eq!((OMKF_HEADER_BYTES, OMKF_SLOT_BYTES), (64, 928));
        let s1 = compute_entry_seal_u64("a.flac", 1024, 999);
        let s2 = compute_entry_seal_u64("b.mp4", 2048, 777);
        let ts = compute_tree_seal_u64(&[s1, s2]);
        assert_eq!(ts, compute_tree_seal_u64(&[s2, s1]));
        let mut slot = OmkfTrackSlot::default();
        (slot.bpm_x100, slot.camelot_code, slot.scan_level) = (12600, camelot_to_code("8A"), 2);
        (slot.genre_family_code, slot.inferred_era_year) = (4, 1994);
        slot.hot_cues_ms[7] = 180000;
        (slot.embedding_64d[0], slot.rgb_peaks_256[767]) = (120, 250);
        assert_eq!((slot.to_bytes()[88], slot.to_bytes()[927]), (4, 250));
        let mut hdr = OmkfHeader::default();
        let mask = camelot_code_mask_bit(slot.camelot_code);
        (hdr.tree_seal_64, hdr.bpm_min_x100, hdr.bpm_max_x100) = (ts, 12600, 12600);
        hdr.camelot_mask = mask;
        hdr.centroid_top24_i8 = compute_centroid_top24_i8(&[slot.clone()]);
        let dir = std::env::temp_dir().join(format!("omkf_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let raw = "# Intención y Reglas de Cabina\nSolo Salsa.\n# Ledger Reversible\n- tx#1\n";
        fs::write(dir.join("crate.md"), raw).unwrap();
        let mut m = OmkfCrateMdMetadata::default();
        (m.crate_id, m.tree_seal_64, m.dominant_camelot) = ("c1".into(), ts, "8A".into());
        let mut e = OmkfTrackEntry::default();
        (e.slot, e.filename) = (slot.clone(), "t.flac".into());
        (e.artist, e.title) = ("Tony Vega".into(), "Lo Mio".into());
        write_omkf_capsule_atomic(&dir, &hdr, &[e], &m).unwrap();
        let omkf = dir.join("tracks.omkf");
        assert_eq!(read_omkf_header(&omkf).unwrap().tree_seal_64, ts);
        let d0 = read_omkf_capsule(&omkf).unwrap();
        assert_eq!(
            (&d0.tracks[0].artist[..], d0.tracks[0].slot.hot_cues_ms[7]),
            ("Tony Vega", 180000)
        );
        let md = fs::read_to_string(dir.join("crate.md")).unwrap();
        assert!(md.contains("Solo Salsa.") && md.contains("- tx#1") && md.contains("omkf: 1"));
        let mut c = OmkfCratePruneCandidate::default();
        (c.crate_id, c.bpm_min, c.bpm_max) = ("c1".into(), Some(120.0), Some(130.0));
        (c.camelot_mask, c.centroid_top24_i8) = (mask, hdr.centroid_top24_i8);
        let mut q = [0_i8; 64];
        q[0] = 120;
        let got = prune_crates_stage1(&[c], &q, Some((124.0, 128.0)), mask, 30);
        assert_eq!(got, ["c1"]);
        assert_eq!(rank_slots_stage2(&[slot], &q, 1)[0].0, 0);
        let _ = fs::remove_dir_all(&dir);
    }
}
