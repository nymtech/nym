describe('testing jest config', () => {
  test('jest works with typescript', () => {
    const foo = () => 42;

    expect(foo()).toBe(42);
  });
});
