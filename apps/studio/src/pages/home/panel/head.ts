import type { HomeCard } from "../card.ts";
import { element } from "../../../core/dom.ts";
import { familyIcon } from "../../../components/family/family.ts";

/** The icon, the name and the SKU of the light, in the head of the panel. */
export function head(card: HomeCard): Node[] {
  const shape = element("span", "light-icon");
  shape.append(familyIcon(card.found.family));
  const text = element("div", "home-text");
  text.append(element("h2", "light-title panel-name", card.name), element("span", "home-sku", card.found.sku ?? ""));
  return [shape, text];
}
