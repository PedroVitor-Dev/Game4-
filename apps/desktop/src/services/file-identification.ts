import type { FileFormat, PlatformId } from "@game4/ipc-contracts";
export const formatLabels = {
  unknown: "Unknown",
  ines: "iNES / NES 2.0",
  n64_z64: "Nintendo 64 (big-endian)",
  n64_v64: "Nintendo 64 (byte-swapped)",
  n64_n64: "Nintendo 64 (little-endian)",
  snes_rom: "SNES ROM",
  game_boy_rom: "Game Boy ROM",
  game_boy_color_rom: "Game Boy Color ROM",
  gba_rom: "Game Boy Advance ROM",
  nds_rom: "Nintendo DS ROM",
  n3ds_rom: "Nintendo 3DS ROM",
  cia: "Nintendo CIA",
  nsp: "Nintendo NSP",
  xci: "Nintendo XCI",
  master_system_rom: "Master System ROM",
  game_gear_rom: "Game Gear ROM",
  mega_drive_rom: "Mega Drive ROM",
  pc_engine_rom: "PC Engine ROM",
  adf: "Amiga ADF",
  d64: "Commodore D64",
  zip: "ZIP",
  seven_zip: "7z",
  rar: "RAR",
  gzip: "gzip",
  chd: "CHD",
  iso9660: "ISO 9660",
  cue: "CUE sheet",
  pe: "Portable Executable (PE)",
  elf: "ELF",
  psx_exe: "PS-X EXE",
  xbe: "Xbox XBE",
  doom_wad: "Doom WAD",
  raw_binary: "Binary data (.bin)",
  raw_rom: "Raw ROM (.rom)",
} as const satisfies Record<FileFormat, string>;
export const platformLabels = {
  nes: "Nintendo Entertainment System",
  snes: "Super Nintendo",
  game_boy: "Game Boy",
  game_boy_color: "Game Boy Color",
  game_boy_advance: "Game Boy Advance",
  nintendo_ds: "Nintendo DS",
  nintendo3ds: "Nintendo 3DS",
  nintendo_switch: "Nintendo Switch",
  nintendo64: "Nintendo 64",
  master_system: "Sega Master System",
  game_gear: "Sega Game Gear",
  mega_drive: "Sega Mega Drive / Genesis",
  pc_engine: "PC Engine / TurboGrafx-16",
  amiga: "Commodore Amiga",
  commodore64: "Commodore 64",
  windows: "Windows",
  playstation: "PlayStation",
  xbox: "Xbox",
  android: "Android",
} as const satisfies Record<PlatformId, string>;

export function formatBytes(value: string, locale: string): string {
  if (!/^(0|[1-9][0-9]*)$/.test(value))
    throw new RangeError("Invalid byte count");
  const bytes = BigInt(value);
  const units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];
  let unit = 0;
  let divisor = 1n;
  while (unit < units.length - 1 && bytes >= divisor * 1024n) {
    unit++;
    divisor *= 1024n;
  }
  if (unit === 0) return `${bytes.toLocaleString(locale)} B`;
  let scaled = (bytes * 100n + divisor / 2n) / divisor;
  if (scaled >= 102400n && unit < units.length - 1) {
    unit++;
    divisor *= 1024n;
    scaled = (bytes * 100n + divisor / 2n) / divisor;
  }
  const fraction = (scaled % 100n)
    .toString()
    .padStart(2, "0")
    .replace(/0+$/, "");
  const separator =
    new Intl.NumberFormat(locale)
      .formatToParts(1.1)
      .find((p) => p.type === "decimal")?.value ?? ".";
  return `${(scaled / 100n).toLocaleString(locale)}${fraction ? separator + fraction : ""} ${units[unit]}`;
}
