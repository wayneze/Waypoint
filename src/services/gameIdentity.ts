/** 仅比较完整名称和括号内别名，不做包含匹配，避免把续作归到本篇。 */
export function gameTitleKeys(title: string): string[] {
  const text = title.normalize('NFKC');
  const aliases = Array.from(text.matchAll(/\(([^)]*)\)/g), match => match[1]!)
    .filter(value => !/^(?:steam|switch|pc|ps[345]|xbox|remaster(?:ed)?|重制|重製|汉化|漢化|高清|完整版|限定版|合集|版本)/i.test(value.trim()));
  return [text, text.replace(/\([^)]*\)/g, ''), ...aliases]
    .map(value => value.toLocaleLowerCase().replace(/[^\p{L}\p{N}]/gu, ''))
    .filter(value => value.length >= 3);
}
