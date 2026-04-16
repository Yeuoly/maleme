import { getCollection, type CollectionEntry } from "astro:content";
import type { Locale } from "./i18n";

export async function listTarotCards() {
  const entries = await getCollection("tarot");
  return entries.sort((left, right) => left.data.order - right.data.order);
}

export async function getTarotCard(slug: string) {
  const cards = await listTarotCards();
  return cards.find((card) => card.id === slug) ?? null;
}

export function getTarotCardCopy(card: CollectionEntry<"tarot">, locale: Locale) {
  return {
    title: card.data.title,
    code: card.data.code,
    summary: locale === "zh-CN" ? card.data.summaryZh : card.data.summary,
    omen: locale === "zh-CN" ? card.data.omenZh : card.data.omen,
  };
}

export function getTarotCardHref(card: Pick<CollectionEntry<"tarot">, "id">) {
  return `/cards/${card.id}`;
}
