import type { Map } from 'maplibre-gl';

const DISCARDED_ELEMENT_NAMES = new Set([
  'AUDIO',
  'CANVAS',
  'EMBED',
  'IFRAME',
  'IMG',
  'MATH',
  'NOSCRIPT',
  'OBJECT',
  'SCRIPT',
  'STYLE',
  'SVG',
  'TEMPLATE',
  'VIDEO',
]);

export type AttributionPart = {
  text: string;
  href?: string;
};

export type ParsedAttribution = {
  source: string;
  parts: AttributionPart[];
};

export function collectMapSourceAttributions(map: Map | undefined): string[] {
  if (!map) return [];

  let style = map.getStyle();
  if (!style?.sources) return [];

  let seen = new Set<string>();
  let attributions: string[] = [];
  for (let sourceId of Object.keys(style.sources)) {
    let attribution = map.getSource(sourceId)?.attribution?.trim();
    if (!attribution || seen.has(attribution)) continue;

    seen.add(attribution);
    attributions.push(attribution);
  }
  return attributions;
}

export function parseMapAttributions(attributions: string[]): ParsedAttribution[] {
  let seen = new Set<string>();
  let parsedAttributions: ParsedAttribution[] = [];

  for (let attribution of attributions) {
    let source = attribution.trim();
    if (!source || seen.has(source)) continue;
    seen.add(source);

    let parts = parseAttribution(source);
    if (parts.some((part) => part.text.trim())) {
      parsedAttributions.push({ source, parts });
    }
  }

  return parsedAttributions;
}

function parseAttribution(attribution: string): AttributionPart[] {
  let document = new DOMParser().parseFromString(attribution, 'text/html');
  let parts: AttributionPart[] = [];
  appendNodes(document.body.childNodes, undefined, parts);
  return parts;
}

function appendNodes(
  nodes: NodeListOf<ChildNode>,
  href: string | undefined,
  parts: AttributionPart[],
) {
  for (let node of nodes) {
    if (node.nodeType === Node.TEXT_NODE) {
      appendText(node.textContent ?? '', href, parts);
      continue;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) continue;

    let element = node as Element;
    if (DISCARDED_ELEMENT_NAMES.has(element.tagName)) continue;
    if (element.tagName === 'BR') {
      appendText(' ', href, parts);
      continue;
    }

    let childHref = element.tagName === 'A' ? resolveHttpUrl(element.getAttribute('href')) : href;
    appendNodes(element.childNodes, childHref, parts);
  }
}

function appendText(text: string, href: string | undefined, parts: AttributionPart[]) {
  if (!text) return;

  let previousPart = parts[parts.length - 1];
  if (previousPart && previousPart.href === href) {
    previousPart.text += text;
    return;
  }

  parts.push({ text, ...(href && { href }) });
}

function resolveHttpUrl(value: string | null): string | undefined {
  if (!value) return undefined;

  try {
    let url = new URL(value, window.location.href);
    return url.protocol === 'http:' || url.protocol === 'https:' ? url.href : undefined;
  } catch {
    return undefined;
  }
}
