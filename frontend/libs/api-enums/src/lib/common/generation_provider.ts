
// NOTE: These are defined in Rust (as the source of truth) and duplicated in the frontend.
// In the future, we should use code gen (protobufs or similar) to keep the two sides in sync.

export enum GenerationProvider {
  Artcraft = "artcraft",
  Grok = "grok",
  Midjourney = "midjourney",
  N8n = "n8n",
  Sora = "sora",
  WorldLabs = "world_labs",
  // NB: We should build our own open source Fal
  //Fal = "fal",
}
