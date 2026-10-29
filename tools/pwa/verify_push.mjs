// Проверка шифрования уведомлений независимой реализацией.
//
// Подписчика играет Node: своя пара ключей и секрет `auth`. Сервер (пример `push_encrypt`)
// шифрует для него сообщение по RFC 8291; здесь оно расшифровывается средствами Node
// (ECDH, HKDF, AES-GCM). Модульный тест в `src/push.rs` проверяет только согласие кода с
// самим собой; эта проверка — с чужой реализацией.
//
// Запуск из корня проекта: node tools/pwa/verify_push.mjs
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const b64u = (b) => Buffer.from(b).toString('base64url');

const ua = crypto.createECDH('prime256v1');
ua.generateKeys();
const uaPub = ua.getPublicKey();
const auth = crypto.randomBytes(16);
const text = 'Новая работа — «Ведьма» · New work';

const out = execFileSync(
  'cargo',
  ['run', '--offline', '-q', '--example', 'push_encrypt', '--', b64u(uaPub), b64u(auth), text],
  { cwd: path.join(root, 'src-tauri/server') },
)
  .toString()
  .trim();

const body = Buffer.from(out, 'base64url');
const salt = body.subarray(0, 16);
const recordSize = body.readUInt32BE(16);
const idLength = body[20];
const asPub = body.subarray(21, 21 + idLength);
const record = body.subarray(21 + idLength);

const secret = ua.computeSecret(asPub);
const keyInfo = Buffer.concat([Buffer.from('WebPush: info\0'), uaPub, asPub]);
const ikm = Buffer.from(crypto.hkdfSync('sha256', secret, auth, keyInfo, 32));
const cek = Buffer.from(
  crypto.hkdfSync('sha256', ikm, salt, Buffer.from('Content-Encoding: aes128gcm\0'), 16),
);
const nonce = Buffer.from(
  crypto.hkdfSync('sha256', ikm, salt, Buffer.from('Content-Encoding: nonce\0'), 12),
);

const decipher = crypto.createDecipheriv('aes-128-gcm', cek, nonce);
decipher.setAuthTag(record.subarray(record.length - 16));
const plain = Buffer.concat([
  decipher.update(record.subarray(0, record.length - 16)),
  decipher.final(),
]);

const problems = [];
if (recordSize !== 4096) problems.push(`размер записи ${recordSize}, ожидалось 4096`);
if (idLength !== 65) problems.push(`длина ключа сервера ${idLength}, ожидалось 65`);
if (record.length > recordSize) problems.push('запись длиннее объявленного размера');
if (plain[plain.length - 1] !== 0x02) problems.push('нет разделителя последней записи (0x02)');
if (plain.subarray(0, plain.length - 1).toString() !== text) problems.push('текст не совпал');

if (problems.length) {
  console.error('НЕ СОШЛОСЬ:\n- ' + problems.join('\n- '));
  process.exit(1);
}
console.log('сошлось: Node расшифровал сообщение, записанное сервером (RFC 8291, aes128gcm)');
