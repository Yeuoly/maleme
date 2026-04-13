import type { APIRoute } from "astro";
import { getSessionCookieName } from "../../../lib/auth";

export const prerender = false;

export const GET: APIRoute = ({ cookies, redirect }) => {
  cookies.delete(getSessionCookieName(), { path: "/" });
  return redirect("/?state=signed-out");
};
