import { describe, it, expect } from 'vitest';
// @ts-expect-error - plain ESM JS module, no type declarations
import { items, matchSymbol, pathSegments } from './items.mjs';

const SOURCE = `use std::fmt;

/// A thing.
#[derive(Debug)]
pub struct Thing {
    pub x: u8,
}

// a plain comment ends the doc run

/// Detached by a blank line.

pub fn free() {}

impl Thing {
    /// Make one.
    pub fn new() -> Self {
        Thing { x: 0 }
    }
}

impl fmt::Display for Thing {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.x)
    }
}

impl fmt::Debug for Other {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

pub trait Sender {
    /// Send it.
    async fn send(&self) -> Result<()>;
}

pub mod inner {
    pub struct Thing;

    impl Thing {
        pub fn new() -> Self { Thing }
    }
}

macro_rules! made {
    () => {};
}
made!();
`;

const names = (list: { qualifiedName: string }[]) => list.map((i) => i.qualifiedName);

describe('items (rust)', async () => {
  const list = await items(SOURCE, 'rust');

  it('finds a whole item with its doc comments and attributes', () => {
    const thing = list.find((i: { qualifiedName: string }) => i.qualifiedName === 'Thing');
    expect(thing).toMatchObject({ kind: 'struct', startLine: 3, endLine: 7 });
  });

  it('does not join a doc comment across a blank line', () => {
    expect(matchSymbol(list, 'free')[0]).toMatchObject({ startLine: 13, endLine: 13 });
  });

  it('names a method by its type, and a trait-impl method in both forms', () => {
    expect(names(matchSymbol(list, 'Thing::new'))).toEqual(['Thing::new', 'inner::Thing::new']);
    expect(matchSymbol(list, 'Thing::fmt')).toHaveLength(1);
    expect(names(matchSymbol(list, '<Thing as fmt::Display>::fmt'))).toEqual(['<Thing as fmt::Display>::fmt']);
  });

  it('names a trait method and an item in a nested module', () => {
    expect(matchSymbol(list, 'Sender::send')[0]).toMatchObject({ kind: 'fn', startLine: 35, endLine: 36 });
    expect(names(matchSymbol(list, 'inner::Thing::new'))).toEqual(['inner::Thing::new']);
  });

  it('gives every candidate for an ambiguous symbol, and none for a missing one', () => {
    expect(matchSymbol(list, 'fmt')).toHaveLength(2);
    expect(matchSymbol(list, 'missing')).toEqual([]);
  });

  it('lists a macro_rules definition but not the items a macro generates', () => {
    expect(matchSymbol(list, 'made')[0]).toMatchObject({ kind: 'macro' });
    expect(list.filter((i: { name: string }) => i.name === 'made')).toHaveLength(1);
  });

  it('splits a path on :: outside angle brackets', () => {
    expect(pathSegments('m::<A as From<b::C>>::from')).toEqual(['m', '<A as From<b::C>>', 'from']);
  });
});

describe('items (rust) review cases', async () => {
  const src = [
    'pub struct W<T>(T);',
    'impl<T> Clone for W<T> {',
    '    fn clone(&self) -> Self { todo!() }',
    '}',
    'impl fmt::Display for W<u8> {',
    '    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { Ok(()) }',
    '}',
    '/// Doc.',
    '#[inline] pub fn inline_attr() {}',
    '/**/',
    'pub fn plain_block() {}',
    'extern "C" {',
    '    fn ffi_call(x: i32) -> i32;',
    '}',
    'pub trait Tr {',
    '    type Assoc;',
    '}',
    'mod inner {',
    '    impl super::W<u8> {',
    '        pub fn m(&self) {}',
    '    }',
    '}',
  ].join('\n');
  const list = await items(src, 'rust');
  const one = (symbol: string) => matchSymbol(list, symbol);

  it('resolves the fully qualified form of a generic type, as the ambiguity error spells it', () => {
    expect(one('<W<T> as Clone>::clone')).toHaveLength(1);
    expect(one('<W<T>  as  Clone>::clone')).toHaveLength(1);
    for (const item of list) expect(one(item.qualifiedName)).toContain(item);
  });

  it('accepts the last segment of a trait path', () => {
    expect(one('<W<u8> as Display>::fmt')).toHaveLength(1);
    expect(one('<W<u8> as fmt::Display>::fmt')).toHaveLength(1);
  });

  it('joins a doc comment above an attribute on the item row, and not an empty block comment', () => {
    expect(one('inline_attr')[0]).toMatchObject({ startLine: 8, endLine: 9 });
    expect(one('plain_block')[0]).toMatchObject({ startLine: 11, endLine: 11 });
  });

  it('finds an extern function, a trait associated type, and a method on a super:: type', () => {
    expect(one('ffi_call')).toHaveLength(1);
    expect(one('Tr::Assoc')[0]).toMatchObject({ kind: 'type' });
    expect(one('inner::W::m')).toHaveLength(1);
  });
});

