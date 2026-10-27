import { defineCollection } from "astro:content";
import { z } from "astro/zod";
import { docsLoader, i18nLoader } from "@astrojs/starlight/loaders";
import { docsSchema, i18nSchema } from "@astrojs/starlight/schema";

export const collections = {
  docs: defineCollection({
    loader: docsLoader(),
    schema: docsSchema(),
  }),
  // Textos dos componentes próprios (topo, telas do app), traduzidos em src/content/i18n/<idioma>.json.
  i18n: defineCollection({
    loader: i18nLoader(),
    schema: i18nSchema({
      extend: z.object({
        "ds.language": z.string().optional(),
        "ds.theme": z.string().optional(),
        "ds.soon": z.string().optional(),
        "ds.pages": z.string().optional(),
        "ds.preview": z.string().optional(),
      }),
    }),
  }),
};
