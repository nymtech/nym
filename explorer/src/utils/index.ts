export function formatNumber(num: number) {
  return new Intl.NumberFormat().format(num);
}

export function scrollToRef(ref: any) {
  return ref.current.scrollIntoView();
}