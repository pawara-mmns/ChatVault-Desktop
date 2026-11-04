// Deterministic, nonprivate test assets. No downloads or real recordings.
import { writeFileSync } from 'node:fs';
import { deflateSync } from 'node:zlib';
const target = new URL('../src/test/fixtures/', import.meta.url);
function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let k = 0; k < 8; k++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}
function chunk(name, body) {
  const type = Buffer.from(name);
  const result = Buffer.alloc(body.length + 12);
  result.writeUInt32BE(body.length, 0);
  type.copy(result, 4);
  body.copy(result, 8);
  result.writeUInt32BE(crc32(Buffer.concat([type, body])), 8 + body.length);
  return result;
}
const width = 160,
  height = 100;
const pixels = Buffer.alloc(height * (width * 3 + 1));
for (let y = 0; y < height; y++)
  for (let x = 0; x < width; x++) {
    const i = y * (width * 3 + 1) + 1 + x * 3;
    const color = ((x >> 4) + (y >> 4)) % 2 ? [125, 190, 155] : [35, 75, 65];
    pixels.set(color, i);
  }
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(width);
ihdr.writeUInt32BE(height, 4);
ihdr[8] = 8;
ihdr[9] = 2;
const png = Buffer.concat([
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  chunk('IHDR', ihdr),
  chunk('IDAT', deflateSync(pixels)),
  chunk('IEND', Buffer.alloc(0)),
]);
const sampleRate = 8000,
  seconds = 4,
  samples = sampleRate * seconds;
const wav = Buffer.alloc(44 + samples * 2);
wav.write('RIFF');
wav.writeUInt32LE(wav.length - 8, 4);
wav.write('WAVEfmt ', 8);
wav.writeUInt32LE(16, 16);
wav.writeUInt16LE(1, 20);
wav.writeUInt16LE(1, 22);
wav.writeUInt32LE(sampleRate, 24);
wav.writeUInt32LE(sampleRate * 2, 28);
wav.writeUInt16LE(2, 32);
wav.writeUInt16LE(16, 34);
wav.write('data', 36);
wav.writeUInt32LE(samples * 2, 40);
for (let n = 0; n < samples; n++)
  wav.writeInt16LE(
    Math.round(2200 * Math.sin((2 * Math.PI * 440 * n) / sampleRate)),
    44 + n * 2,
  );
// Deliberately truncated ISO-BMFF fixture: recognizable container signature,
// no decodable video track. Tests graceful WebView decode-error feedback.
const badVideo = Buffer.alloc(24);
badVideo.writeUInt32BE(24);
badVideo.write('ftypisom', 4);
badVideo.write('isommp42', 16);
const text = Buffer.from(
  '31/12/2026, 09:30 - Fixture Alice: checker.png (file attached)\n31/12/2026, 09:31 - Fixture Bob: tone.wav (file attached)\n31/12/2026, 09:32 - Fixture Alice: unsupported.mp4 (file attached)\n31/12/2026, 09:33 - Fixture Bob: Gr\u00fc\u00dfe offline waveform fixture\n',
  'utf8',
);
function zip(files) {
  const entries = [],
    central = [];
  let offset = 0;
  for (const [name, body] of files) {
    const filename = Buffer.from(name);
    const crc = crc32(body);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(0x800, 6);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(body.length, 18);
    local.writeUInt32LE(body.length, 22);
    local.writeUInt16LE(filename.length, 26);
    entries.push(local, filename, body);
    const directory = Buffer.alloc(46);
    directory.writeUInt32LE(0x02014b50);
    directory.writeUInt16LE(20, 4);
    directory.writeUInt16LE(20, 6);
    directory.writeUInt16LE(0x800, 8);
    directory.writeUInt32LE(crc, 16);
    directory.writeUInt32LE(body.length, 20);
    directory.writeUInt32LE(body.length, 24);
    directory.writeUInt16LE(filename.length, 28);
    directory.writeUInt32LE(offset, 42);
    central.push(directory, filename);
    offset += local.length + filename.length + body.length;
  }
  const index = Buffer.concat(central);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50);
  end.writeUInt16LE(files.length, 8);
  end.writeUInt16LE(files.length, 10);
  end.writeUInt32LE(index.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...entries, index, end]);
}
writeFileSync(
  new URL('SyntheticViewerMedia.zip', target),
  zip([
    ['_chat.txt', text],
    ['media/checker.png', png],
    ['media/tone.wav', wav],
    ['media/unsupported.mp4', badVideo],
  ]),
);
console.log(
  `Created synthetic ZIP: ${width}x${height} PNG, ${seconds}s PCM WAV, and a deliberately unsupported video fixture.`,
);
