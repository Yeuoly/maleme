import type { APIRoute } from "astro";
import {
  getGitHubAuthorizeUrl,
  getSessionCookieName,
  hasGitHubAuthConfig,
  isSecureRequest,
} from "../../../../lib/auth";

export const prerender = false;

export const GET: APIRoute = ({ cookies, redirect, url }) => {
  if (!hasGitHubAuthConfig()) {
    return redirect("/?state=auth-misconfigured");
  }

  const state = crypto.randomUUID();
  cookies.set("maleme_oauth_state", state, {
    httpOnly: true,
    maxAge: 600,
    path: "/",
    sameSite: "lax",
    secure: isSecureRequest(url),
  });
  cookies.delete(getSessionCookieName(), { path: "/" });
  return redirect(getGitHubAuthorizeUrl(url, state));
};
