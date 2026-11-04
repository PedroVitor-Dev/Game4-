-- Signature observations are tied to bytes, never to mutable filename hints.
CREATE TABLE file_identifications (
    file_id INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    format_id TEXT NOT NULL CHECK (format_id IN ('unknown','ines','n64_z64','n64_v64','n64_n64','zip','seven_zip','rar','gzip','chd','iso9660','pe','elf','psx_exe','xbe','doom_wad')),
    platform_id TEXT,
    detector_version INTEGER NOT NULL CHECK (detector_version = 1),
    CHECK (platform_id IS CASE format_id WHEN 'ines' THEN 'nes' WHEN 'n64_z64' THEN 'nintendo64' WHEN 'n64_v64' THEN 'nintendo64' WHEN 'n64_n64' THEN 'nintendo64' WHEN 'psx_exe' THEN 'playstation' WHEN 'xbe' THEN 'xbox' ELSE NULL END)
) STRICT;
