import { defineCollection } from "astro:content";
import { z } from "astro/zod";
import { docsLoader } from "@astrojs/starlight/loaders";
import { docsSchema } from "@astrojs/starlight/schema";

export const collections = {
  docs: defineCollection({
    loader: docsLoader(),
    schema: docsSchema({
      extend: z.object({
        /** Brilho em gradiente (L78) atrás do título: ocean, aurora, sunset, meadow ou potato. Só em páginas de destaque. */
        glow: z.enum(["ocean", "aurora", "sunset", "meadow", "potato"]).optional(),
      }),
    }),
  }),
};
