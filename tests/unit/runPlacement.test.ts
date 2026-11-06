// Запускается Node напрямую (`npm run test:unit`): путь с `.ts` ему нужен, а svelte-check его не любит.
// @ts-nocheck
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { autoLayout, moveTo, reconcile, threatened, toRequest } from '../../src/lib/runPlacement.ts';

const open = [
  { x: 1, y: 4 },
  { x: 1, y: 3 },
  { x: 0, y: 4 },
  { x: 2, y: 4 },
];

test('without memory cards take open cells in the order the server names', () => {
  assert.deepEqual(autoLayout(['a', 'b'], open), { a: { x: 1, y: 4 }, b: { x: 1, y: 3 } });
});

test('a remembered open cell is kept and the rest fill the first free ones', () => {
  const got = reconcile(['a', 'b'], {}, { a: { x: 2, y: 4 } }, open);
  assert.deepEqual(got.a, { x: 2, y: 4 });
  assert.deepEqual(got.b, { x: 1, y: 4 });
});

test('a remembered cell that closed is replaced by the first free one', () => {
  const got = reconcile(['a'], {}, { a: { x: 9, y: 9 } }, open);
  assert.deepEqual(got.a, { x: 1, y: 4 });
});

test('two cards that remember one cell: the first keeps it, the second moves', () => {
  const got = reconcile(['a', 'b'], {}, { a: { x: 0, y: 4 }, b: { x: 0, y: 4 } }, open);
  assert.deepEqual(got.a, { x: 0, y: 4 });
  assert.notDeepEqual(got.b, { x: 0, y: 4 });
});

test('the current choice beats the memory', () => {
  const got = reconcile(['a'], { a: { x: 2, y: 4 } }, { a: { x: 0, y: 4 } }, open);
  assert.deepEqual(got.a, { x: 2, y: 4 });
});

test('a card that finds no room is left without a cell', () => {
  const got = reconcile(['a', 'b', 'c', 'd', 'e'], {}, {}, open);
  assert.equal(Object.keys(got).length, 4);
  assert.equal(got.e, undefined);
});

test('moving onto a free open cell moves the card', () => {
  const cells = autoLayout(['a'], open);
  assert.deepEqual(moveTo(cells, 'a', { x: 2, y: 4 }, open).a, { x: 2, y: 4 });
});

test('moving onto another card swaps them', () => {
  const cells = autoLayout(['a', 'b'], open);
  const got = moveTo(cells, 'a', { x: 1, y: 3 }, open);
  assert.deepEqual(got.a, { x: 1, y: 3 });
  assert.deepEqual(got.b, { x: 1, y: 4 });
});

test('moving onto a closed cell changes nothing', () => {
  const cells = autoLayout(['a'], open);
  assert.equal(moveTo(cells, 'a', { x: 5, y: 5 }, open), cells);
});

test('threatened cells are the union of every zone', () => {
  const got = threatened([{ cells: [{ x: 1, y: 3 }] }, { cells: [{ x: 1, y: 3 }, { x: 2, y: 4 }] }]);
  assert.deepEqual([...got].sort(), ['1,3', '2,4']);
});

test('the request names a cell for every placed card and none for the rest', () => {
  const req = toRequest(['a', 'b'], { a: { x: 1, y: 4 } });
  assert.deepEqual(req, [{ cardId: 'a', x: 1, y: 4 }, { cardId: 'b' }]);
});
