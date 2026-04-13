import type { APIRoute } from "astro";
import {
  createSessionToken,
  decodePendingSubmission,
  exchangeCodeForToken,
  fetchGitHubUser,
  getCookieMaxAge,
  getPendingSubmissionCookieName,
  getSessionCookieName,
  isSecureRequest,
  sessionCookieSameSite,
} from "../../../../lib/auth";
import { consumePendingSubmission, hasDatabaseBinding, upsertLeaderboardEntry } from "../../../../lib/db";

export const prerender = false;

export const GET: APIRoute = async ({ cookies, redirect, url }) => {
  const code = url.searchParams.get("code");
  const state = url.searchParams.get("state");
  const expectedState = cookies.get("maleme_oauth_state")?.value;

  if (!code || !state || !expectedState || state !== expectedState) {
    return redirect("/?state=oauth-denied");
  }

  try {
    const token = await exchangeCodeForToken(url, code);
    const viewer = await fetchGitHubUser(token);
    const sessionToken = await createSessionToken(viewer);

    cookies.set(getSessionCookieName(), sessionToken, {
      httpOnly: true,
      maxAge: getCookieMaxAge(),
      path: "/",
      sameSite: sessionCookieSameSite(url),
      secure: isSecureRequest(url),
    });
    cookies.delete("maleme_oauth_state", { path: "/" });

    const pendingValue = cookies.get(getPendingSubmissionCookieName())?.value;

    if (pendingValue && hasDatabaseBinding()) {
      const pending =
        (await consumePendingSubmission(pendingValue)) ??
        (() => {
          try {
            return decodePendingSubmission(pendingValue);
          } catch {
            return null;
          }
        })();

      cookies.delete(getPendingSubmissionCookieName(), { path: "/" });

      if (pending) {
        await upsertLeaderboardEntry(viewer, pending);
        return redirect(`/u/${encodeURIComponent(viewer.login)}?state=submitted`);
      }
    }

    return redirect("/?state=signed-in");
  } catch {
    return redirect("/?state=oauth-failed");
  }
};
