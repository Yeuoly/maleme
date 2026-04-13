import type { APIRoute } from "astro";
import {
  getPendingSubmissionCookieName,
  getPendingSubmissionMaxAge,
  getSessionCookieName,
  isSecureRequest,
  readViewerFromToken,
  sessionCookieSameSite,
} from "../lib/auth";
import { createFallbackReportPayload, parseReportPayloadJson } from "../lib/report";
import { createPendingSubmission, hasDatabaseBinding, upsertLeaderboardEntry } from "../lib/db";

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

  const fallback = createFallbackReportPayload({
    profanityCount: Math.trunc(profanityCount),
    tokens: Math.trunc(tokens),
    sbai,
  });
  const reportPayload = parseReportPayloadJson(formData.get("reportPayload")?.toString(), fallback);

  const token = cookies.get(getSessionCookieName())?.value;
  const viewer = await readViewerFromToken(token).catch(() => null);

  if (!viewer) {
    const pendingToken = await createPendingSubmission(reportPayload, getPendingSubmissionMaxAge() * 1000);

    cookies.set(
      getPendingSubmissionCookieName(),
      pendingToken,
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

  await upsertLeaderboardEntry(viewer, reportPayload);
  return redirect(`/u/${encodeURIComponent(viewer.login)}?state=submitted`);
};
