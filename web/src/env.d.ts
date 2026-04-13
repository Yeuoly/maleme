type D1Database = any;
type Fetcher = any;

type WorkerEnv = {
  ASSETS: Fetcher;
  APP_URL?: string;
  DB?: D1Database;
  GITHUB_CLIENT_ID?: string;
  GITHUB_CLIENT_SECRET?: string;
  SESSION_SECRET?: string;
};

declare module "cloudflare:workers" {
  export const env: WorkerEnv;
}
