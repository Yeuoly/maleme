import { defineCollection, z } from "astro:content";
import { glob } from "astro/loaders";

const tarot = defineCollection({
  loader: glob({
    pattern: "**/*.md",
    base: "./src/content/tarot",
  }),
  schema: z.object({
    order: z.number().int().positive(),
    title: z.string(),
    code: z.string(),
    summary: z.string(),
    omen: z.string(),
  }),
});

export const collections = {
  tarot,
};
