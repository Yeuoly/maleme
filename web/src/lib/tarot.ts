import { getCollection, type CollectionEntry } from "astro:content";

export async function listTarotCards() {
  const entries = await getCollection("tarot");
  return entries.sort((left, right) => left.data.order - right.data.order);
}

export async function getTarotCard(slug: string) {
  const cards = await listTarotCards();
  return cards.find((card) => card.slug === slug) ?? null;
}

export function getTarotCardHref(card: Pick<CollectionEntry<"tarot">, "slug">) {
  return `/cards/${card.slug}`;
}
