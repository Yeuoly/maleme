import type { APIRoute } from "astro";
import {
  encodePendingSubmission,
  getPendingSubmissionCookieName,
  getPendingSubmissionMaxAge,
  getSessionCookieName,
  isSecureRequest,
  readViewerFromToken,
  sessionCookieSameSite,
} from "../lib/auth";
import { hasDatabaseBinding, upsertLeaderboardEntry } from "../lib/db";

export const prerender = false;

export const POST: APIRoute = async ({ cookies, redirect, request, url }) => {
  if (!hasDatabaseBinding()) {
    return redirect("/");
  }

  const formData = await request.formData();
  const profanityCount = Number(formData.get("profanityCount"));
  const tokens = Number(formData.get("tokens"));
  const sbai = Number(formData.get("sbai"));

  if (!Number.isFinite(profanityCount) || profanityCount < 0) {
    return redirect("/");
  }

  if (!Number.isFinite(tokens) || tokens < 0) {
    return redirect("/");
  }

  if (!Number.isFinite(sbai) || sbai < 0) {
    return redirect("/");
  }

  const token = cookies.get(getSessionCookieName())?.value;
  const viewer = await readViewerFromToken(token).catch(() => null);

  if (!viewer) {
    cookies.set(
      getPendingSubmissionCookieName(),
      encodePendingSubmission({
        profanityCount: Math.trunc(profanityCount),
        tokens: Math.trunc(tokens),
        sbai,
      }),
      {
        httpOnly: true,
        maxAge: getPendingSubmissionMaxAge(),
        path: "/",
        sameSite: sessionCookieSameSite(url),
        secure: isSecureRequest(url),
      },
    );
    return redirect("/api/auth/github/login");
  }

  await upsertLeaderboardEntry(viewer, Math.trunc(profanityCount), Math.trunc(tokens), sbai);
  return redirect("/");
};
