//! Explicit metainfo/payload binding; called only for a user-selected torrent.
use super::*;
struct Decoder<'a> {
    raw: &'a [u8],
    at: usize,
    info: Option<(usize, usize)>,
}
impl Decoder<'_> {
    fn value(&mut self, depth: usize) -> Result<B> {
        if depth > 64 {
            return Err(limit());
        }
        let c = *self.raw.get(self.at).ok_or_else(invalid)?;
        match c {
            b'i' => {
                self.at += 1;
                let start = self.at;
                while self.raw.get(self.at) != Some(&b'e') {
                    if self.at >= self.raw.len() {
                        return Err(invalid());
                    }
                    self.at += 1;
                }
                let text = std::str::from_utf8(&self.raw[start..self.at]).map_err(|_| invalid())?;
                let n = text.parse::<i128>().map_err(|_| invalid())?;
                if n.to_string() != text {
                    return Err(invalid());
                }
                self.at += 1;
                Ok(B::Int(n))
            }
            b'l' => {
                self.at += 1;
                let mut items = Vec::new();
                while self.raw.get(self.at) != Some(&b'e') {
                    items.push(self.value(depth + 1)?);
                }
                self.at += 1;
                Ok(B::List(items))
            }
            b'd' => {
                self.at += 1;
                let mut items = BTreeMap::new();
                let mut last: Option<Vec<u8>> = None;
                while self.raw.get(self.at) != Some(&b'e') {
                    let B::Bytes(k) = self.value(depth + 1)? else {
                        return Err(invalid());
                    };
                    if last.as_ref().is_some_and(|v| v >= &k) {
                        return Err(invalid());
                    }
                    last = Some(k.clone());
                    let start = self.at;
                    let v = self.value(depth + 1)?;
                    if depth == 0 && k == b"info" {
                        self.info = Some((start, self.at));
                    }
                    items.insert(k, v);
                }
                self.at += 1;
                Ok(B::Dict(items))
            }
            b'0'..=b'9' => {
                let start = self.at;
                while self.raw.get(self.at) != Some(&b':') {
                    if self.at >= self.raw.len() {
                        return Err(invalid());
                    }
                    self.at += 1;
                }
                let text = std::str::from_utf8(&self.raw[start..self.at]).map_err(|_| invalid())?;
                let size = text.parse::<usize>().map_err(|_| invalid())?;
                if size.to_string() != text {
                    return Err(invalid());
                }
                self.at += 1;
                let end = self
                    .at
                    .checked_add(size)
                    .filter(|n| *n <= self.raw.len())
                    .ok_or_else(invalid)?;
                let b = self.raw[self.at..end].to_vec();
                self.at = end;
                Ok(B::Bytes(b))
            }
            _ => Err(invalid()),
        }
    }
}
fn map(b: &B) -> Result<&BTreeMap<Vec<u8>, B>> {
    if let B::Dict(v) = b {
        Ok(v)
    } else {
        Err(invalid())
    }
}
fn get<'a>(m: &'a BTreeMap<Vec<u8>, B>, k: &[u8]) -> Result<&'a B> {
    m.get(k).ok_or_else(invalid)
}
fn bytes(b: &B) -> Result<&[u8]> {
    if let B::Bytes(v) = b {
        Ok(v)
    } else {
        Err(invalid())
    }
}
fn text(b: &B) -> Result<&str> {
    std::str::from_utf8(bytes(b)?).map_err(|_| invalid())
}
fn number(b: &B) -> Result<u64> {
    if let B::Int(v) = b {
        u64::try_from(*v).map_err(|_| invalid())
    } else {
        Err(invalid())
    }
}
fn regular(m: &BTreeMap<Vec<u8>, B>) -> Result<()> {
    if m.contains_key(b"symlink path".as_slice())
        || m.get(b"attr".as_slice())
            .is_some_and(|v| !matches!(v,B::Bytes(b) if b.is_empty()))
    {
        return Err(invalid());
    }
    Ok(())
}
pub(super) struct Bound {
    files: Vec<ManifestFile>,
    piece_length: usize,
    pieces: Vec<u8>,
}
pub(super) fn bind(doc: &EverManifest, raw: &[u8]) -> Result<Bound> {
    let total = validate(doc)?;
    if raw.len() > MAX_METAINFO {
        return Err(limit());
    }
    let mut decoder = Decoder {
        raw,
        at: 0,
        info: None,
    };
    let top = decoder.value(0)?;
    if decoder.at != raw.len() {
        return Err(invalid());
    }
    let (start, end) = decoder.info.ok_or_else(invalid)?;
    let hash = format!("{:x}", Sha1::digest(&raw[start..end]));
    let reference = doc
        .torrents
        .iter()
        .find(|t| t.info_hash_v1 == hash)
        .ok_or_else(integrity)?;
    if let Some(companion) = &reference.metainfo_file
        && (companion.size_bytes != raw.len().to_string()
            || companion.sha256 != format!("{:x}", Sha256::digest(raw)))
    {
        return Err(integrity());
    }
    let info = map(get(map(&top)?, b"info")?)?;
    regular(info)?;
    if info.contains_key(b"meta version".as_slice())
        || info
            .get(b"private".as_slice())
            .is_some_and(|v| !matches!(v, B::Int(0)))
    {
        return Err(invalid());
    }
    let name = text(get(info, b"name")?)?;
    path(name)?;
    if name.contains('/') {
        return Err(invalid());
    }
    let piece_length =
        usize::try_from(number(get(info, b"piece length")?)?).map_err(|_| limit())?;
    if !(16 * 1024..=16 * 1024 * 1024).contains(&piece_length) || !piece_length.is_power_of_two() {
        return Err(limit());
    }
    let pieces = bytes(get(info, b"pieces")?)?.to_vec();
    if total.div_ceil(piece_length as u64).checked_mul(20) != Some(pieces.len() as u64) {
        return Err(invalid());
    }
    let mut metadata = Vec::new();
    match (
        info.get(b"files".as_slice()),
        info.get(b"length".as_slice()),
    ) {
        (Some(B::List(files)), None) => {
            if files.is_empty() || files.len() > manifests::MAX_FILES {
                return Err(limit());
            }
            for file in files {
                let file = map(file)?;
                regular(file)?;
                let B::List(parts) = get(file, b"path")? else {
                    return Err(invalid());
                };
                if parts.is_empty() || parts.len() > 128 {
                    return Err(invalid());
                }
                let mut path_parts = Vec::new();
                for part in parts {
                    let part = text(part)?;
                    if part.contains('/') {
                        return Err(invalid());
                    }
                    path_parts.push(part);
                }
                let p = path_parts.join("/");
                path(&p)?;
                metadata.push((p, number(get(file, b"length")?)?));
            }
        }
        (None, Some(n)) => metadata.push((name.to_owned(), number(n)?)),
        _ => return Err(invalid()),
    }
    let expected: BTreeMap<_, _> = doc
        .manifest
        .files
        .iter()
        .map(|f| (f.path.as_str(), f))
        .collect();
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for (p, n) in metadata {
        if !seen.insert(p.clone()) {
            return Err(invalid());
        }
        let f = expected.get(p.as_str()).ok_or_else(integrity)?;
        if f.size_bytes != n.to_string() {
            return Err(integrity());
        }
        files.push((*f).clone());
    }
    if files.len() != doc.manifest.files.len() {
        return Err(integrity());
    }
    Ok(Bound {
        files,
        piece_length,
        pieces,
    })
}
/// Verifies explicit selected metainfo and originals, without network or writes.
pub fn verify(doc: &EverManifest, torrent: &[u8], root: &Path) -> Result<()> {
    let bound = bind(doc, torrent)?;
    let root = absolute(root)?;
    let actual = hash_sources(&bound.files, &root, bound.piece_length)?;
    if actual != bound.pieces {
        return Err(integrity());
    }
    if matches!(doc.representation, EverRepresentation::G4Package) {
        let file = source(&root, &bound.files[0], false)?;
        let p = crate::packages::read(&file)?;
        if p.manifest.collection.id != doc.manifest.collection.id {
            return Err(integrity());
        }
    }
    Ok(())
}
