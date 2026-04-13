use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use thiserror::Error;
use time::OffsetDateTime;

use crate::{FuckDetector, UserMessage};

const DAY_MS: i64 = 86_400_000;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DailyCount {
    label: String,
    count: i64,
}

#[derive(Debug, Clone, PartialEq)]
struct ReportData {
    daily_counts: Vec<DailyCount>,
    word_counts: Vec<(String, i64)>,
    total_profanities: i64,
    total_tokens: i64,
    sbai: f64,
    message_count: usize,
    range_start: String,
    range_end: String,
    submit_endpoint: &'static str,
}

struct ReportTheme {
    bg: &'static str,
    bg_accent: &'static str,
    panel: &'static str,
    panel_strong: &'static str,
    panel_tint: &'static str,
    muted: &'static str,
    text: &'static str,
    line: &'static str,
    line_soft: &'static str,
    area_top: &'static str,
    area_bottom: &'static str,
    accent: &'static str,
    accent_soft: &'static str,
    accent_warm: &'static str,
    accent_pink: &'static str,
    border: &'static str,
    grid: &'static str,
    shadow: &'static str,
    tooltip_bg: &'static str,
    hero_from: &'static str,
    hero_to: &'static str,
    hero_glow_a: &'static str,
    hero_glow_b: &'static str,
    cloud_glow: &'static str,
    cloud_to: &'static str,
    sbai_glow: &'static str,
    sbai_surface_top: &'static str,
    sbai_surface_bottom: &'static str,
    sbai_border: &'static str,
    sbai_text: &'static str,
    sbai_muted: &'static str,
    word_palette: [&'static str; 6],
    fireworks_palette: [&'static str; 5],
}

pub fn write_report_and_open(
    messages: &[UserMessage],
    tokens: i64,
    detector: &FuckDetector,
) -> Result<PathBuf, ReportError> {
    let report = render_report(messages, tokens, detector)?;
    let output_path = downloads_dir()?.join(report_filename()?);
    fs::create_dir_all(output_path.parent().unwrap()).map_err(|source| ReportError::Io {
        path: output_path.parent().unwrap().to_path_buf(),
        source,
    })?;
    fs::write(&output_path, report).map_err(|source| ReportError::Io {
        path: output_path.clone(),
        source,
    })?;

    let status = Command::new("open")
        .arg(&output_path)
        .status()
        .map_err(|source| ReportError::OpenBrowser {
            path: output_path.clone(),
            source,
        })?;

    if !status.success() {
        return Err(ReportError::OpenBrowserStatus {
            path: output_path,
            code: status.code().unwrap_or(-1),
        });
    }

    Ok(output_path)
}

pub fn render_report(
    messages: &[UserMessage],
    tokens: i64,
    detector: &FuckDetector,
) -> Result<String, ReportError> {
    let data = build_report_data(messages, tokens, detector)?;
    let chart = render_line_chart(&data.daily_counts);
    let word_cloud = render_word_cloud(&data.word_counts)?;
    let (sbai_state, sbai_copy) = sbai_state_copy(data.sbai);
    let theme = report_theme(data.sbai);
    let word_palette_json = serde_json::to_string(&theme.word_palette)
        .map_err(ReportError::WordCloudJson)?
        .replace("</", "<\\/");
    let fireworks_palette_json = serde_json::to_string(&theme.fireworks_palette)
        .map_err(ReportError::WordCloudJson)?
        .replace("</", "<\\/");

    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN" data-word-palette='{word_palette_json}' data-fireworks-palette='{fireworks_palette_json}'>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>maleme report</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Orbitron:wght@700;800;900&family=Rajdhani:wght@500;600;700&family=Noto+Sans+SC:wght@500;700;900&display=swap" rel="stylesheet">
  <script src="https://cdn.jsdelivr.net/npm/echarts@6.0.0/dist/echarts.min.js"></script>
  <style>
    :root {{
      color-scheme: light;
      --bg: {bg};
      --bg-accent: {bg_accent};
      --panel: {panel};
      --panel-strong: {panel_strong};
      --panel-tint: {panel_tint};
      --muted: {muted};
      --text: {text};
      --line: {line};
      --line-soft: {line_soft};
      --area-top: {area_top};
      --area-bottom: {area_bottom};
      --accent: {accent};
      --accent-soft: {accent_soft};
      --accent-warm: {accent_warm};
      --accent-pink: {accent_pink};
      --border: {border};
      --grid: {grid};
      --shadow: {shadow};
      --tooltip-bg: {tooltip_bg};
      --hero-from: {hero_from};
      --hero-to: {hero_to};
      --hero-glow-a: {hero_glow_a};
      --hero-glow-b: {hero_glow_b};
      --cloud-glow: {cloud_glow};
      --cloud-to: {cloud_to};
      --sbai-glow: {sbai_glow};
      --sbai-surface-top: {sbai_surface_top};
      --sbai-surface-bottom: {sbai_surface_bottom};
      --sbai-border: {sbai_border};
      --sbai-text: {sbai_text};
      --sbai-muted: {sbai_muted};
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: "Rajdhani", "Noto Sans SC", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background:
        radial-gradient(circle at top left, var(--cloud-glow), transparent 24%),
        radial-gradient(circle at top right, var(--line-soft), transparent 24%),
        linear-gradient(180deg, var(--bg) 0%, var(--bg-accent) 100%);
      color: var(--text);
      height: 100vh;
      overflow: hidden;
    }}
    .page {{
      position: relative;
      z-index: 1;
      max-width: 1320px;
      margin: 0 auto;
      height: 100vh;
      padding: 18px 18px 20px;
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
      gap: 16px;
      overflow: hidden;
    }}
    .hero {{
      display: grid;
      grid-template-columns: minmax(0, 1.55fr) minmax(360px, 1.05fr);
      gap: 16px;
      align-items: stretch;
      min-height: 0;
    }}
    .panel {{
      background:
        linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0)),
        var(--panel);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 16px;
      box-shadow: var(--shadow);
      backdrop-filter: blur(12px);
      min-height: 0;
    }}
    .hero-main {{
      position: relative;
      overflow: hidden;
      background: linear-gradient(135deg, var(--hero-from), var(--hero-to));
    }}
    .hero-main::before {{
      content: "";
      position: absolute;
      inset: 0 auto auto 0;
      width: 180px;
      height: 180px;
      background: radial-gradient(circle, var(--hero-glow-a), transparent 70%);
      transform: translate(-30%, -30%);
    }}
    .hero-main::after {{
      content: "";
      position: absolute;
      inset: auto 0 0 auto;
      width: 200px;
      height: 200px;
      background: radial-gradient(circle, var(--hero-glow-b), transparent 70%);
      transform: translate(30%, 30%);
    }}
    .headline-wrap {{
      position: relative;
      z-index: 3;
    }}
    .fireworks {{
      position: absolute;
      inset: 0;
      width: 100%;
      height: 100%;
      pointer-events: none;
      z-index: 2;
    }}
    h1 {{
      margin: 0;
      font-size: 48px;
      line-height: 0.98;
      font-weight: 900;
      font-family: "Orbitron", "Noto Sans SC", sans-serif;
      max-width: 13ch;
      text-wrap: balance;
      animation: headline-pop 2.4s ease-in-out infinite;
    }}
    .headline-count {{
      color: var(--line);
      text-shadow: 0 10px 22px var(--line-soft);
    }}
    h2 {{
      margin: 0 0 14px;
      font-size: 20px;
      line-height: 1.2;
      font-family: "Orbitron", "Noto Sans SC", sans-serif;
    }}
    .subtle {{
      color: var(--muted);
      font-size: 14px;
      line-height: 1.5;
    }}
    .meta {{
      display: flex;
      flex-wrap: wrap;
      gap: 10px;
      margin-top: 14px;
      margin-bottom: 14px;
    }}
    .meta-item {{
      padding: 10px 12px;
      border-radius: 8px;
      background: var(--panel-tint);
      border: 1px solid var(--border);
      min-width: 132px;
    }}
    .meta-label {{
      color: var(--muted);
      font-size: 12px;
      margin-bottom: 6px;
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }}
    .meta-value {{
      font-size: 22px;
      font-weight: 700;
      font-family: "Orbitron", "Noto Sans SC", sans-serif;
      font-variant-numeric: tabular-nums;
    }}
    .submit-form {{
      position: relative;
      z-index: 3;
      margin-top: 6px;
    }}
    .submit-button {{
      height: 48px;
      border: 0;
      border-radius: 8px;
      padding: 0 18px;
      font: inherit;
      font-size: 15px;
      font-weight: 900;
      color: #ffffff;
      background: linear-gradient(135deg, #ff5a5f, #ff7f50);
      box-shadow: 0 16px 28px rgba(255, 90, 95, 0.24);
      cursor: pointer;
    }}
    .submit-button:hover {{
      transform: translateY(-1px);
    }}
    .sbai-card {{
      position: relative;
      display: flex;
      flex-direction: column;
      gap: 12px;
      min-height: 100%;
      overflow: hidden;
      background:
        radial-gradient(circle at 16% 16%, var(--sbai-glow), transparent 24%),
        linear-gradient(180deg, rgba(255, 245, 238, 0.05), rgba(255, 245, 238, 0) 22%),
        linear-gradient(156deg, var(--sbai-surface-top) 0%, #141318 36%, var(--sbai-surface-bottom) 100%);
      border-color: var(--sbai-border);
      box-shadow:
        0 24px 48px rgba(8, 9, 14, 0.42),
        inset 0 0 0 1px rgba(255, 245, 238, 0.06),
        inset 0 0 52px rgba(255, 35, 70, 0.12);
    }}
    .sbai-card::before {{
      content: "";
      position: absolute;
      inset: 0;
      background:
        linear-gradient(180deg, rgba(255, 255, 255, 0.03), transparent 36%, rgba(255, 255, 255, 0.02) 72%, transparent 100%),
        repeating-linear-gradient(
          180deg,
          rgba(255, 255, 255, 0.05) 0,
          rgba(255, 255, 255, 0.05) 1px,
          transparent 1px,
          transparent 7px
        );
      opacity: 0.42;
      pointer-events: none;
    }}
    .sbai-card::after {{
      content: "";
      position: absolute;
      inset: -20% 0 auto;
      height: 44%;
      background: linear-gradient(180deg, rgba(255, 73, 95, 0), rgba(255, 73, 95, 0.14), rgba(255, 73, 95, 0));
      filter: blur(8px);
      opacity: 0.72;
      animation: sbai-scan 6s linear infinite;
      pointer-events: none;
    }}
    .sbai-card > * {{
      position: relative;
      z-index: 1;
    }}
    .sbai-header {{
      display: flex;
      align-items: flex-start;
      justify-content: space-between;
      gap: 12px;
    }}
    .sbai-label {{
      color: var(--sbai-text);
      font-size: 13px;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      font-weight: 800;
    }}
    .sbai-alert {{
      flex-shrink: 0;
      padding: 6px 9px;
      border-radius: 4px;
      border: 1px solid var(--sbai-border);
      background: var(--line-soft);
      color: var(--sbai-text);
      font-size: 12px;
      font-weight: 800;
      line-height: 1;
      text-transform: uppercase;
      box-shadow: inset 0 0 18px rgba(255, 62, 110, 0.12);
      animation: sbai-pulse 2.8s ease-in-out infinite;
    }}
    .sbai-kicker {{
      max-width: 13ch;
      color: var(--sbai-text);
      font-size: 15px;
      line-height: 1.2;
      font-weight: 800;
      text-transform: uppercase;
    }}
    .sbai-value-wrap {{
      padding-top: 6px;
    }}
    .sbai-value {{
      position: relative;
      display: inline-block;
      font-size: 92px;
      line-height: 0.88;
      font-weight: 900;
      font-family: "Orbitron", "Noto Sans SC", sans-serif;
      margin: 0;
      color: var(--sbai-text);
      font-variant-numeric: tabular-nums;
      text-shadow:
        0 0 22px rgba(255, 81, 96, 0.24),
        0 0 40px rgba(255, 81, 96, 0.08);
      animation: sbai-jolt 3.6s steps(2, end) infinite;
    }}
    .sbai-value::before,
    .sbai-value::after {{
      content: attr(data-display);
      position: absolute;
      inset: 0;
      pointer-events: none;
    }}
    .sbai-value::before {{
      color: var(--line);
      transform: translate(-3px, -1px);
      opacity: 0.74;
    }}
    .sbai-value::after {{
      color: var(--accent-warm);
      transform: translate(3px, 1px);
      opacity: 0.6;
    }}
    .sbai-mantra {{
      max-width: 11ch;
      color: var(--sbai-text);
      font-size: 28px;
      line-height: 1.04;
      font-weight: 900;
      font-family: "Orbitron", "Noto Sans SC", sans-serif;
      text-transform: uppercase;
      text-wrap: balance;
    }}
    .sbai-copy {{
      max-width: 22ch;
      color: var(--sbai-muted);
      font-size: 14px;
      line-height: 1.42;
      font-weight: 600;
    }}
    .sbai-divider {{
      width: 100%;
      height: 1px;
      margin-top: auto;
      background: linear-gradient(90deg, var(--sbai-text), rgba(255, 237, 223, 0.08));
    }}
    .sbai-chant {{
      color: var(--sbai-text);
      font-size: 12px;
      line-height: 1.2;
      font-weight: 800;
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }}
    .sbai-footnote {{
      color: var(--sbai-muted);
      font-size: 13px;
      line-height: 1.3;
    }}
    .layout {{
      display: grid;
      grid-template-columns: minmax(0, 0.95fr) minmax(0, 1.05fr);
      gap: 16px;
      min-height: 0;
      align-items: stretch;
    }}
    .viz-panel {{
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
      min-height: 0;
    }}
    .chart-panel {{
      min-width: 0;
    }}
    .cloud-panel {{
      min-width: 0;
    }}
    .chart-wrap {{
      width: 100%;
      height: 100%;
      min-height: 0;
      overflow: hidden;
    }}
    .chart-host {{
      width: 100%;
      height: 100%;
      min-height: 320px;
    }}
    .chart-fallback {{
      display: none;
      place-items: center;
      width: 100%;
      height: 100%;
      min-height: 320px;
      color: var(--muted);
      background: var(--panel-strong);
      border-radius: 8px;
      text-align: center;
      padding: 24px;
    }}
    .cloud {{
      position: relative;
      width: 100%;
      height: 100%;
      min-height: 360px;
      overflow: hidden;
    }}
    .cloud-viewport {{
      position: relative;
      width: 100%;
      height: 100%;
      min-height: inherit;
      overflow: hidden;
      border-radius: 8px;
      background:
        radial-gradient(circle at top left, var(--cloud-glow), transparent 28%),
        linear-gradient(180deg, var(--panel-strong), var(--cloud-to));
      cursor: grab;
      touch-action: none;
    }}
    .cloud-viewport.is-dragging {{
      cursor: grabbing;
    }}
    .cloud-scene {{
      position: absolute;
      inset: 0;
      width: 100%;
      height: 100%;
      display: block;
    }}
    .cloud-fallback {{
      display: none;
      position: absolute;
      inset: 0;
      place-items: center;
      padding: 24px;
      text-align: center;
      color: var(--muted);
      background: var(--panel-strong);
      z-index: 3;
    }}
    .empty {{
      color: var(--muted);
      font-size: 16px;
      padding: 32px 0;
    }}
    @keyframes headline-pop {{
      0%, 100% {{
        transform: translateY(0) scale(1);
        text-shadow: 0 0 0 rgba(255, 183, 3, 0);
      }}
      50% {{
        transform: translateY(-2px) scale(1.01);
        text-shadow: 0 10px 18px rgba(255, 183, 3, 0.26);
      }}
    }}
    .chart-line {{
      stroke-dasharray: 1600;
      stroke-dashoffset: 1600;
      animation: line-draw 1.4s ease-out forwards;
    }}
    .chart-fill {{
      opacity: 0;
      animation: fade-fill 1.2s ease-out 0.2s forwards;
    }}
    .chart-dot {{
      opacity: 0;
      transform-origin: center;
      animation: dot-pop 0.35s ease-out forwards;
    }}
    @keyframes line-draw {{
      to {{ stroke-dashoffset: 0; }}
    }}
    @keyframes fade-fill {{
      to {{ opacity: 1; }}
    }}
    @keyframes dot-pop {{
      0% {{
        opacity: 0;
        transform: scale(0.4);
      }}
      100% {{
        opacity: 1;
        transform: scale(1);
      }}
    }}
    @keyframes sbai-scan {{
      0% {{ transform: translateY(-30%); }}
      100% {{ transform: translateY(240%); }}
    }}
    @keyframes sbai-pulse {{
      0%, 100% {{
        transform: translateY(0);
        box-shadow: inset 0 0 18px rgba(255, 62, 110, 0.12);
      }}
      50% {{
        transform: translateY(-1px);
        box-shadow: inset 0 0 24px rgba(255, 62, 110, 0.22), 0 0 18px rgba(255, 62, 110, 0.12);
      }}
    }}
    @keyframes sbai-jolt {{
      0%, 86%, 100% {{
        transform: translate3d(0, 0, 0);
      }}
      88% {{
        transform: translate3d(-1px, 0, 0);
      }}
      90% {{
        transform: translate3d(1px, -1px, 0);
      }}
      92% {{
        transform: translate3d(-2px, 1px, 0);
      }}
      94% {{
        transform: translate3d(2px, 0, 0);
      }}
      96% {{
        transform: translate3d(-1px, -1px, 0);
      }}
    }}
    @media (max-width: 900px) {{
      body {{ height: auto; overflow: auto; }}
      .page {{ height: auto; padding: 16px; overflow: visible; }}
      .hero {{ grid-template-columns: 1fr; }}
      .layout {{ grid-template-columns: 1fr; }}
      h1 {{ font-size: 42px; max-width: 100%; }}
      .sbai-value {{ font-size: 68px; }}
      .sbai-copy {{ max-width: none; }}
      .sbai-mantra {{ max-width: none; font-size: 24px; }}
    }}
  </style>
</head>
<body>
  <div class="page">
    <section class="hero">
      <div class="panel hero-main">
        <canvas id="fireworks" class="fireworks"></canvas>
        <div class="headline-wrap">
        <h1>{headline}</h1>
        <div class="meta">
          <div class="meta-item">
            <div class="meta-label">聊天输入</div>
            <div class="meta-value number-roll" data-target-number="{message_count}" data-decimals="0">0</div>
          </div>
          <div class="meta-item">
            <div class="meta-label">脏话次数</div>
            <div class="meta-value number-roll" data-target-number="{total_profanities}" data-decimals="0">0</div>
          </div>
          <div class="meta-item">
            <div class="meta-label">总 Tokens</div>
            <div class="meta-value number-roll" data-target-number="{total_tokens}" data-decimals="0">0</div>
          </div>
        </div>
        <form class="submit-form" method="post" action="{submit_endpoint}">
          <input type="hidden" name="profanityCount" value="{total_profanities}">
          <input type="hidden" name="tokens" value="{total_tokens}">
          <input type="hidden" name="sbai" value="{sbai:.2}">
          <button type="submit" class="submit-button">提交到 leaderboard 看看你有多能骂！</button>
        </form>
        </div>
      </div>
      <div class="panel sbai-card">
        <div class="sbai-header">
          <div class="sbai-label">SBAI 指数</div>
          <div class="sbai-alert">{sbai_state}</div>
        </div>
        <div class="sbai-kicker">AI 写得越自信</div>
        <div class="sbai-value-wrap">
          <div class="sbai-value number-roll" data-target-number="{sbai:.2}" data-decimals="2" data-display="0.00">0.00</div>
        </div>
        <div class="sbai-mantra">人越接近发疯</div>
        <div class="sbai-copy">{sbai_copy}</div>
        <div class="sbai-divider"></div>
        <div class="sbai-chant">乱写 / 破防 / 暴走</div>
        <div class="sbai-footnote">每千万 tokens 的骂人次数</div>
      </div>
    </section>

    <section class="layout">
      <div class="panel viz-panel chart-panel">
        <h2>你这一天骂了 AI 多少次！</h2>
        <div class="chart-wrap">{chart}</div>
      </div>

      <div class="panel viz-panel cloud-panel">
        <h2>你最喜欢这么骂！</h2>
        {word_cloud}
      </div>
    </section>
  </div>
  <script>
    (() => {{
      const rootStyles = getComputedStyle(document.documentElement);
      const theme = {{
        line: rootStyles.getPropertyValue('--line').trim(),
        lineSoft: rootStyles.getPropertyValue('--line-soft').trim(),
        areaTop: rootStyles.getPropertyValue('--area-top').trim(),
        areaBottom: rootStyles.getPropertyValue('--area-bottom').trim(),
        accent: rootStyles.getPropertyValue('--accent').trim(),
        accentSoft: rootStyles.getPropertyValue('--accent-soft').trim(),
        border: rootStyles.getPropertyValue('--border').trim(),
        grid: rootStyles.getPropertyValue('--grid').trim(),
        muted: rootStyles.getPropertyValue('--muted').trim(),
        tooltipBg: rootStyles.getPropertyValue('--tooltip-bg').trim(),
        panelStrong: rootStyles.getPropertyValue('--panel-strong').trim()
      }};
      const canvas = document.getElementById("fireworks");
      const host = canvas.parentElement;
      const ctx = canvas.getContext("2d");
      const particles = [];
      const palette = JSON.parse(document.documentElement.dataset.fireworksPalette || '[]');
      let fireworksTimer = null;

      function resize() {{
        const rect = host.getBoundingClientRect();
        const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
        canvas.width = rect.width * dpr;
        canvas.height = rect.height * dpr;
        canvas.style.width = rect.width + "px";
        canvas.style.height = rect.height + "px";
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      }}

      function burst() {{
        const rect = host.getBoundingClientRect();
        const x = 60 + Math.random() * Math.max(rect.width - 120, 60);
        const y = 30 + Math.random() * Math.min(rect.height * 0.5, 180);
        const count = 40;
        for (let i = 0; i < count; i += 1) {{
          const angle = (Math.PI * 2 * i) / count;
          const speed = 1.7 + Math.random() * 3.2;
          particles.push({{
            x,
            y,
            vx: Math.cos(angle) * speed,
            vy: Math.sin(angle) * speed,
            life: 48 + Math.random() * 24,
            color: palette[i % palette.length],
            size: 2.1 + Math.random() * 2.8
          }});
        }}
      }}

      function tick() {{
        const rect = host.getBoundingClientRect();
        ctx.clearRect(0, 0, rect.width, rect.height);
        for (let i = particles.length - 1; i >= 0; i -= 1) {{
          const particle = particles[i];
          particle.x += particle.vx;
          particle.y += particle.vy;
          particle.vy += 0.035;
          particle.life -= 1;
          if (particle.life <= 0) {{
            particles.splice(i, 1);
            continue;
          }}
          ctx.globalAlpha = particle.life / 68;
          ctx.fillStyle = particle.color;
          ctx.beginPath();
          ctx.arc(particle.x, particle.y, particle.size, 0, Math.PI * 2);
          ctx.fill();
        }}
        ctx.globalAlpha = 1;
        requestAnimationFrame(tick);
      }}

      resize();
      burst();
      burst();
      burst();
      fireworksTimer = setInterval(() => {{
        burst();
        if (Math.random() > 0.55) {{
          burst();
        }}
      }}, 1100);
      setTimeout(() => {{
        if (fireworksTimer) {{
          clearInterval(fireworksTimer);
        }}
      }}, 14000);
      window.addEventListener("resize", resize);
      tick();

      function formatNumber(value, decimals) {{
        return value.toLocaleString('en-US', {{
          minimumFractionDigits: decimals,
          maximumFractionDigits: decimals
        }});
      }}

      function animateNumbers() {{
        const nodes = document.querySelectorAll('.number-roll');
        const duration = 1400;
        const start = performance.now();

        function frame(now) {{
          const progress = Math.min((now - start) / duration, 1);
          const eased = 1 - Math.pow(1 - progress, 3);

          nodes.forEach((node) => {{
            const target = Number(node.dataset.targetNumber || '0');
            const decimals = Number(node.dataset.decimals || '0');
            const value = target * eased;
            const formatted = formatNumber(value, decimals);
            node.textContent = formatted;
            node.dataset.display = formatted;
          }});

          if (progress < 1) {{
            requestAnimationFrame(frame);
          }}
        }}

        requestAnimationFrame(frame);
      }}

      animateNumbers();

      function initTrendChart() {{
        const host = document.getElementById('daily-chart');
        const fallback = document.getElementById('daily-chart-fallback');

        if (!host) {{
          return;
        }}

        const points = JSON.parse(host.dataset.points || '[]');
        if (!points.length) {{
          return;
        }}

        try {{
          if (typeof echarts === 'undefined') {{
            throw new Error('echarts is not available');
          }}

          const chart = echarts.init(host, null, {{ renderer: 'canvas' }});
          const labels = points.map(point => point.label);
          const values = points.map(point => point.count);

          chart.setOption({{
            animationDuration: 700,
            animationEasing: 'cubicOut',
            tooltip: {{
              trigger: 'axis',
              backgroundColor: theme.tooltipBg,
              borderWidth: 0,
              textStyle: {{
                color: '#ffffff'
              }},
              formatter(params) {{
                const point = params[0];
                return `${{point.axisValue}}<br/>你这一天骂了 AI ${{point.data}} 次`;
              }}
            }},
            grid: {{
              left: 46,
              right: 18,
              top: 16,
              bottom: 34,
              containLabel: true
            }},
            xAxis: {{
              type: 'category',
              boundaryGap: false,
              data: labels,
              axisLine: {{
                lineStyle: {{ color: theme.border }}
              }},
              axisTick: {{ show: false }},
              axisLabel: {{
                color: theme.muted,
                hideOverlap: true
              }}
            }},
            yAxis: {{
              type: 'value',
              minInterval: 1,
              axisLine: {{ show: false }},
              axisTick: {{ show: false }},
              splitLine: {{
                lineStyle: {{ color: theme.grid }}
              }},
              axisLabel: {{
                color: theme.muted
              }}
            }},
            dataZoom: [
              {{
                type: 'inside',
                xAxisIndex: 0,
                filterMode: 'none',
                moveOnMouseMove: true,
                moveOnMouseWheel: true,
                zoomOnMouseWheel: true
              }},
              {{
                type: 'slider',
                xAxisIndex: 0,
                filterMode: 'none',
                height: 18,
                bottom: 4,
                borderColor: theme.border,
                backgroundColor: theme.panelStrong,
                fillerColor: theme.accentSoft,
                handleSize: 14,
                textStyle: {{
                  color: theme.muted
                }}
              }}
            ],
            series: [{{
              type: 'line',
              data: values,
              smooth: 0.22,
              symbol: 'circle',
              symbolSize: 8,
              showSymbol: true,
              sampling: 'lttb',
              lineStyle: {{
                color: theme.line,
                width: 3
              }},
              itemStyle: {{
                color: theme.accent,
                borderColor: '#ffffff',
                borderWidth: 2
              }},
              areaStyle: {{
                color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
                  {{ offset: 0, color: theme.areaTop }},
                  {{ offset: 1, color: theme.areaBottom }}
                ])
              }}
            }}]
          }});

          const resizeChart = () => chart.resize();
          window.addEventListener('resize', resizeChart);
          window.addEventListener('pagehide', () => {{
            window.removeEventListener('resize', resizeChart);
            chart.dispose();
          }}, {{ once: true }});
        }} catch (error) {{
          console.error(error);
          if (fallback) {{
            fallback.style.display = 'grid';
          }}
        }}
      }}

      function initWordSphere() {{
        const viewport = document.getElementById('word-cloud-viewport');
        const sceneHost = document.getElementById('word-cloud-scene');
        const fallback = document.getElementById('word-cloud-fallback');

        if (!viewport || !sceneHost) {{
          return;
        }}

        const words = JSON.parse(sceneHost.dataset.words || '[]');
        if (!words.length) {{
          return;
        }}

        try {{
          const ctx = sceneHost.getContext('2d');
          if (!ctx) {{
            throw new Error('2d canvas context is not available');
          }}

          const handleResize = () => {{
            resizeScene();
          }};
          const resizeObserver = typeof ResizeObserver === 'function'
            ? new ResizeObserver(handleResize)
            : null;
          let isInteracting = false;
          let width = 0;
          let height = 0;
          let radius = 180;
          let zoom = 1;
          let animationFrame = 0;
          let rotationX = -0.18;
          let rotationY = 0;
          let rotationVelocityX = 0.00024;
          let rotationVelocityY = 0.0019;
          let lastPointerX = 0;
          let lastPointerY = 0;
          let lastRenderAt = 0;
          const palette = JSON.parse(document.documentElement.dataset.wordPalette || '[]');
          const entries = buildEntries(words);

          function buildEntries(list) {{
            const maxCount = list.reduce((maxValue, entry) => Math.max(maxValue, entry[1]), 1);
            const total = list.length;
            const goldenAngle = Math.PI * (3 - Math.sqrt(5));

            return list.map((entry, index) => {{
              const [label, count] = entry;
              const normalized = maxCount <= 1
                ? 0.6
                : Math.pow((count - 1) / Math.max(maxCount - 1, 1), 0.72);
              const y = 1 - (index / Math.max(total - 1, 1)) * 2;
              const ring = Math.sqrt(Math.max(0, 1 - y * y));
              const theta = goldenAngle * index;

              return {{
                label,
                color: palette[index % palette.length],
                weight: normalized,
                x: Math.cos(theta) * ring,
                y,
                z: Math.sin(theta) * ring,
                sprite: createWordSprite(label, normalized, palette[index % palette.length])
              }};
            }});
          }}

          function createWordSprite(label, weight, color) {{
            const baseFontSize = 44 + weight * 34;
            const paddingX = 24;
            const paddingY = 18;
            const ratio = 2;
            const spriteCanvas = document.createElement('canvas');
            const spriteCtx = spriteCanvas.getContext('2d');
            spriteCtx.font = `800 ${{baseFontSize}}px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`;
            const metrics = spriteCtx.measureText(label);
            const logicalWidth = Math.ceil(metrics.width + paddingX * 2);
            const logicalHeight = Math.ceil(baseFontSize + paddingY * 2);

            spriteCanvas.width = logicalWidth * ratio;
            spriteCanvas.height = logicalHeight * ratio;

            const drawCtx = spriteCanvas.getContext('2d');
            drawCtx.scale(ratio, ratio);
            drawCtx.font = `800 ${{baseFontSize}}px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`;
            drawCtx.textAlign = 'center';
            drawCtx.textBaseline = 'middle';
            drawCtx.lineJoin = 'round';
            drawCtx.lineWidth = Math.max(6, baseFontSize * 0.12);
            drawCtx.strokeStyle = 'rgba(255,255,255,0.94)';
            drawCtx.shadowColor = 'rgba(20, 33, 43, 0.14)';
            drawCtx.shadowBlur = 12;
            drawCtx.strokeText(label, logicalWidth / 2, logicalHeight / 2);
            drawCtx.shadowBlur = 0;
            drawCtx.fillStyle = color;
            drawCtx.fillText(label, logicalWidth / 2, logicalHeight / 2);

            return {{
              canvas: spriteCanvas,
              width: logicalWidth,
              height: logicalHeight
            }};
          }}

          function resizeScene() {{
            const rect = viewport.getBoundingClientRect();
            width = Math.max(Math.round(rect.width), 320);
            height = Math.max(Math.round(rect.height), 260);
            const ratio = Math.min(window.devicePixelRatio || 1, 2);
            sceneHost.width = Math.round(width * ratio);
            sceneHost.height = Math.round(height * ratio);
            sceneHost.style.width = width + 'px';
            sceneHost.style.height = height + 'px';
            ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
            radius = Math.max(110, Math.min(width, height) * 0.34);
          }}

          function clamp(value, min, max) {{
            return Math.min(Math.max(value, min), max);
          }}

          function wrapAngle(value) {{
            const fullTurn = Math.PI * 2;
            if (value > fullTurn || value < -fullTurn) {{
              return value % fullTurn;
            }}
            return value;
          }}

          function rotatePoint(point) {{
            const cosY = Math.cos(rotationY);
            const sinY = Math.sin(rotationY);
            const x1 = point.x * cosY - point.z * sinY;
            const z1 = point.x * sinY + point.z * cosY;
            const cosX = Math.cos(rotationX);
            const sinX = Math.sin(rotationX);
            const y2 = point.y * cosX - z1 * sinX;
            const z2 = point.y * sinX + z1 * cosX;

            return {{ x: x1, y: y2, z: z2 }};
          }}

          function projectEntry(entry) {{
            const rotated = rotatePoint(entry);
            const depth = (rotated.z + 1) / 2;

            return {{
              ...entry,
              x: width / 2 + rotated.x * radius * zoom,
              y: height / 2 + rotated.y * radius * zoom * 0.9,
              z: rotated.z,
              alpha: 0.1 + depth * 0.8,
              depth,
              scale: (0.36 + depth * 0.84) * (0.72 + entry.weight * 0.55) * zoom
            }};
          }}

          function drawEntry(entry) {{
            if (entry.depth < 0.08 || entry.scale < 0.22) {{
              return;
            }}

            ctx.save();
            ctx.globalAlpha = entry.alpha;
            const drawWidth = entry.sprite.width * entry.scale;
            const drawHeight = entry.sprite.height * entry.scale;
            ctx.drawImage(
              entry.sprite.canvas,
              entry.x - drawWidth / 2,
              entry.y - drawHeight / 2,
              drawWidth,
              drawHeight
            );
            ctx.restore();
          }}

          function renderSphere() {{
            ctx.clearRect(0, 0, width, height);

            ctx.save();
            ctx.beginPath();
            ctx.arc(width / 2, height / 2, radius * zoom * 1.04, 0, Math.PI * 2);
            ctx.fillStyle = 'rgba(255,255,255,0.12)';
            ctx.fill();
            ctx.restore();

            entries
              .map(projectEntry)
              .sort((left, right) => left.z - right.z)
              .forEach(drawEntry);
          }}

          function handlePointerDown(event) {{
            if (event.button !== 0) {{
              return;
            }}

            isInteracting = true;
            lastPointerX = event.clientX;
            lastPointerY = event.clientY;
            viewport.classList.add('is-dragging');
            viewport.setPointerCapture(event.pointerId);
          }}

          function handlePointerMove(event) {{
            if (!isInteracting) {{
              return;
            }}

            const deltaX = event.clientX - lastPointerX;
            const deltaY = event.clientY - lastPointerY;
            lastPointerX = event.clientX;
            lastPointerY = event.clientY;

            rotationX = wrapAngle(rotationX - deltaY * 0.006);
            rotationY = wrapAngle(rotationY - deltaX * 0.006);
            rotationVelocityX = -deltaY * 0.00045;
            rotationVelocityY = -deltaX * 0.00045;
          }}

          function releasePointer(event) {{
            if (!isInteracting) {{
              return;
            }}

            isInteracting = false;
            viewport.classList.remove('is-dragging');
            if (viewport.hasPointerCapture(event.pointerId)) {{
              viewport.releasePointerCapture(event.pointerId);
            }}
          }}

          function handleWheel(event) {{
            event.preventDefault();
            const zoomFactor = event.deltaY > 0 ? 0.94 : 1.06;
            zoom = clamp(zoom * zoomFactor, 0.82, 1.72);
          }}

          function renderFrame(now) {{
            animationFrame = requestAnimationFrame(renderFrame);
            const targetFrameGap = isInteracting ? 1000 / 60 : 1000 / 24;
            if (now - lastRenderAt < targetFrameGap) {{
              return;
            }}
            lastRenderAt = now;
            if (!isInteracting) {{
              rotationX = wrapAngle(rotationX + rotationVelocityX);
              rotationY = wrapAngle(rotationY + rotationVelocityY);
              rotationVelocityX *= 0.92;
              rotationVelocityY *= 0.982;
              if (Math.abs(rotationVelocityY) < 0.0009) {{
                rotationVelocityY = 0.0009;
              }}
            }}
            renderSphere();
          }}

          viewport.addEventListener('pointerdown', handlePointerDown);
          viewport.addEventListener('pointermove', handlePointerMove);
          viewport.addEventListener('pointerup', releasePointer);
          viewport.addEventListener('pointercancel', releasePointer);
          viewport.addEventListener('wheel', handleWheel, {{ passive: false }});

          resizeScene();
          renderSphere();
          if (resizeObserver) {{
            resizeObserver.observe(viewport);
          }} else {{
            window.addEventListener('resize', handleResize);
          }}
          renderFrame();

          window.addEventListener('pagehide', () => {{
            cancelAnimationFrame(animationFrame);
            resizeObserver?.disconnect();
            window.removeEventListener('resize', handleResize);
            viewport.removeEventListener('pointerdown', handlePointerDown);
            viewport.removeEventListener('pointermove', handlePointerMove);
            viewport.removeEventListener('pointerup', releasePointer);
            viewport.removeEventListener('pointercancel', releasePointer);
            viewport.removeEventListener('wheel', handleWheel);
          }}, {{ once: true }});
        }} catch (error) {{
          console.error(error);
          fallback.style.display = 'grid';
        }}
      }}

      initTrendChart();
      initWordSphere();
    }})();
  </script>
</body>
</html>"#,
        message_count = data.message_count,
        total_profanities = data.total_profanities,
        total_tokens = data.total_tokens,
        sbai = data.sbai,
        headline = report_headline(&data.range_start, &data.range_end, data.total_profanities),
        chart = chart,
        word_cloud = word_cloud,
        sbai_state = sbai_state,
        sbai_copy = sbai_copy,
        word_palette_json = word_palette_json,
        fireworks_palette_json = fireworks_palette_json,
        bg = theme.bg,
        bg_accent = theme.bg_accent,
        panel = theme.panel,
        panel_strong = theme.panel_strong,
        panel_tint = theme.panel_tint,
        muted = theme.muted,
        text = theme.text,
        line = theme.line,
        line_soft = theme.line_soft,
        area_top = theme.area_top,
        area_bottom = theme.area_bottom,
        accent = theme.accent,
        accent_soft = theme.accent_soft,
        accent_warm = theme.accent_warm,
        accent_pink = theme.accent_pink,
        border = theme.border,
        grid = theme.grid,
        shadow = theme.shadow,
        tooltip_bg = theme.tooltip_bg,
        hero_from = theme.hero_from,
        hero_to = theme.hero_to,
        hero_glow_a = theme.hero_glow_a,
        hero_glow_b = theme.hero_glow_b,
        cloud_glow = theme.cloud_glow,
        cloud_to = theme.cloud_to,
        sbai_glow = theme.sbai_glow,
        sbai_surface_top = theme.sbai_surface_top,
        sbai_surface_bottom = theme.sbai_surface_bottom,
        sbai_border = theme.sbai_border,
        sbai_text = theme.sbai_text,
        sbai_muted = theme.sbai_muted,
        submit_endpoint = data.submit_endpoint,
    ))
}

fn build_report_data(
    messages: &[UserMessage],
    tokens: i64,
    detector: &FuckDetector,
) -> Result<ReportData, ReportError> {
    if messages.is_empty() {
        return Ok(ReportData {
            daily_counts: Vec::new(),
            word_counts: Vec::new(),
            total_profanities: 0,
            total_tokens: tokens,
            sbai: 0.0,
            message_count: 0,
            range_start: "还没有记录".to_owned(),
            range_end: "还没有记录".to_owned(),
            submit_endpoint: "https://leaderboard.sbai.uk/submit",
        });
    }

    let mut daily_counts = BTreeMap::new();
    let mut word_counts = BTreeMap::new();
    let mut total_profanities = 0_i64;
    let mut min_day = messages[0].time.div_euclid(DAY_MS);
    let mut max_day = min_day;

    for message in messages {
        let day = message.time.div_euclid(DAY_MS);
        let counts = detector.detect(&message.text);
        let mut daily_total = 0_i64;

        if day < min_day {
            min_day = day;
        }

        if day > max_day {
            max_day = day;
        }

        for (word, count) in counts {
            daily_total += count;
            total_profanities += count;
            *word_counts.entry(word).or_insert(0) += count;
        }

        *daily_counts.entry(day).or_insert(0) += daily_total;
    }

    let mut daily_series = Vec::new();
    let range_start = day_label(min_day)?;
    let range_end = day_label(max_day)?;

    for day in min_day..=max_day {
        daily_series.push(DailyCount {
            label: day_label(day)?,
            count: *daily_counts.get(&day).unwrap_or(&0),
        });
    }

    let mut word_series = word_counts.into_iter().collect::<Vec<_>>();
    word_series.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    let sbai = if tokens == 0 {
        0.0
    } else {
        total_profanities as f64 * 10_000_000.0 / tokens as f64
    };

    Ok(ReportData {
        daily_counts: daily_series,
        word_counts: word_series,
        total_profanities,
        total_tokens: tokens,
        sbai,
        message_count: messages.len(),
        range_start,
        range_end,
        submit_endpoint: "https://leaderboard.sbai.uk/submit",
    })
}

fn report_headline(range_start: &str, range_end: &str, total_profanities: i64) -> String {
    if total_profanities < 10 {
        return format!(
            "心如止水。{} 到 {}，你只骂了 AI <span class=\"headline-count\">{}</span> 次。",
            range_start, range_end, total_profanities
        );
    }

    if total_profanities < 100 {
        return format!(
            "喜报！！！你的心态也太平稳了叭！！！！{} 到 {}，只骂了 AI <span class=\"headline-count\">{}</span> 次！",
            range_start, range_end, total_profanities
        );
    }

    format!(
        "喜报！！你从 {} 到 {} 一共骂了 AI <span class=\"headline-count\">{}</span> 次！",
        range_start, range_end, total_profanities
    )
}

fn report_theme(sbai: f64) -> ReportTheme {
    if sbai < 0.5 {
        return ReportTheme {
            bg: "#090d12",
            bg_accent: "#121821",
            panel: "rgba(10, 15, 20, 0.84)",
            panel_strong: "rgba(13, 18, 24, 0.94)",
            panel_tint: "rgba(14, 23, 31, 0.78)",
            muted: "#8ca5af",
            text: "#edf8ff",
            line: "#ffb800",
            line_soft: "rgba(255, 184, 0, 0.18)",
            area_top: "rgba(255, 184, 0, 0.22)",
            area_bottom: "rgba(255, 184, 0, 0.04)",
            accent: "#59e8ff",
            accent_soft: "rgba(89, 232, 255, 0.16)",
            accent_warm: "#ffe85b",
            accent_pink: "#ff5f7d",
            border: "rgba(89, 232, 255, 0.22)",
            grid: "rgba(50, 87, 96, 0.58)",
            shadow: "0 20px 48px rgba(4, 7, 12, 0.34)",
            tooltip_bg: "rgba(6, 10, 16, 0.94)",
            hero_from: "rgba(11, 16, 22, 0.96)",
            hero_to: "rgba(13, 20, 27, 0.96)",
            hero_glow_a: "rgba(255, 232, 91, 0.16)",
            hero_glow_b: "rgba(89, 232, 255, 0.16)",
            cloud_glow: "rgba(89, 232, 255, 0.12)",
            cloud_to: "rgba(10, 18, 25, 0.96)",
            sbai_glow: "rgba(255, 184, 0, 0.24)",
            sbai_surface_top: "#0b0e13",
            sbai_surface_bottom: "#1b1910",
            sbai_border: "rgba(255, 232, 171, 0.24)",
            sbai_text: "rgba(255, 246, 210, 0.96)",
            sbai_muted: "rgba(255, 226, 158, 0.82)",
            word_palette: ["#59e8ff", "#ffb800", "#ffe85b", "#ff5f7d", "#89f5ff", "#ffcf5a"],
            fireworks_palette: ["#59e8ff", "#ffb800", "#ffe85b", "#ff5f7d", "#89f5ff"],
        };
    }

    if sbai < 2.0 {
        return ReportTheme {
            bg: "#0a0d12",
            bg_accent: "#16151f",
            panel: "rgba(12, 15, 21, 0.86)",
            panel_strong: "rgba(14, 18, 25, 0.95)",
            panel_tint: "rgba(25, 22, 30, 0.78)",
            muted: "#9ca0b1",
            text: "#f7f7ff",
            line: "#ff8f1f",
            line_soft: "rgba(255, 143, 31, 0.2)",
            area_top: "rgba(255, 143, 31, 0.24)",
            area_bottom: "rgba(255, 143, 31, 0.05)",
            accent: "#57d8ff",
            accent_soft: "rgba(87, 216, 255, 0.18)",
            accent_warm: "#ffd447",
            accent_pink: "#ff5f7d",
            border: "rgba(104, 143, 166, 0.34)",
            grid: "rgba(56, 74, 92, 0.58)",
            shadow: "0 20px 48px rgba(4, 7, 12, 0.36)",
            tooltip_bg: "rgba(7, 9, 15, 0.94)",
            hero_from: "rgba(14, 18, 24, 0.96)",
            hero_to: "rgba(19, 18, 28, 0.96)",
            hero_glow_a: "rgba(255, 212, 71, 0.16)",
            hero_glow_b: "rgba(87, 216, 255, 0.18)",
            cloud_glow: "rgba(87, 216, 255, 0.12)",
            cloud_to: "rgba(15, 19, 27, 0.96)",
            sbai_glow: "rgba(255, 143, 31, 0.28)",
            sbai_surface_top: "#0b0d12",
            sbai_surface_bottom: "#251717",
            sbai_border: "rgba(255, 192, 108, 0.26)",
            sbai_text: "rgba(255, 236, 199, 0.96)",
            sbai_muted: "rgba(255, 198, 123, 0.84)",
            word_palette: ["#57d8ff", "#ff8f1f", "#ffd447", "#ff5f7d", "#88e6ff", "#ffb356"],
            fireworks_palette: ["#57d8ff", "#ff8f1f", "#ffd447", "#ff5f7d", "#88e6ff"],
        };
    }

    if sbai < 5.0 {
        return ReportTheme {
            bg: "#0b0a0f",
            bg_accent: "#1a1017",
            panel: "rgba(15, 12, 18, 0.88)",
            panel_strong: "rgba(18, 14, 20, 0.96)",
            panel_tint: "rgba(36, 17, 24, 0.8)",
            muted: "#b39aa3",
            text: "#fff3ef",
            line: "#ff5d3d",
            line_soft: "rgba(255, 93, 61, 0.24)",
            area_top: "rgba(255, 93, 61, 0.28)",
            area_bottom: "rgba(255, 93, 61, 0.07)",
            accent: "#4fdbff",
            accent_soft: "rgba(79, 219, 255, 0.16)",
            accent_warm: "#ffd54a",
            accent_pink: "#ff4f7a",
            border: "rgba(118, 69, 84, 0.8)",
            grid: "rgba(77, 41, 55, 0.84)",
            shadow: "0 24px 54px rgba(5, 4, 9, 0.42)",
            tooltip_bg: "rgba(6, 6, 10, 0.96)",
            hero_from: "rgba(18, 13, 19, 0.96)",
            hero_to: "rgba(24, 13, 20, 0.96)",
            hero_glow_a: "rgba(255, 213, 74, 0.14)",
            hero_glow_b: "rgba(79, 219, 255, 0.12)",
            cloud_glow: "rgba(79, 219, 255, 0.1)",
            cloud_to: "rgba(20, 12, 18, 0.96)",
            sbai_glow: "rgba(255, 93, 61, 0.34)",
            sbai_surface_top: "#0a0a0e",
            sbai_surface_bottom: "#4a1017",
            sbai_border: "rgba(255, 173, 118, 0.26)",
            sbai_text: "rgba(255, 238, 202, 0.97)",
            sbai_muted: "rgba(255, 189, 138, 0.86)",
            word_palette: ["#4fdbff", "#ff5d3d", "#ffd54a", "#ff4f7a", "#8beaff", "#ff965a"],
            fireworks_palette: ["#4fdbff", "#ff5d3d", "#ffd54a", "#ff4f7a", "#8beaff"],
        };
    }

    ReportTheme {
        bg: "#07080b",
        bg_accent: "#170b10",
        panel: "rgba(14, 10, 14, 0.9)",
        panel_strong: "rgba(17, 12, 16, 0.97)",
        panel_tint: "rgba(49, 13, 21, 0.78)",
        muted: "#c7a4ac",
        text: "#fff4ef",
        line: "#ff3b30",
        line_soft: "rgba(255, 59, 48, 0.28)",
        area_top: "rgba(255, 59, 48, 0.3)",
        area_bottom: "rgba(255, 59, 48, 0.08)",
        accent: "#46dcff",
        accent_soft: "rgba(70, 220, 255, 0.18)",
        accent_warm: "#ffd93d",
        accent_pink: "#ff3f76",
        border: "rgba(123, 45, 63, 0.86)",
        grid: "rgba(82, 25, 41, 0.84)",
        shadow: "0 26px 58px rgba(3, 3, 6, 0.48)",
        tooltip_bg: "rgba(4, 4, 7, 0.97)",
        hero_from: "rgba(18, 11, 15, 0.97)",
        hero_to: "rgba(22, 11, 16, 0.97)",
        hero_glow_a: "rgba(255, 217, 61, 0.14)",
        hero_glow_b: "rgba(70, 220, 255, 0.1)",
        cloud_glow: "rgba(70, 220, 255, 0.08)",
        cloud_to: "rgba(18, 11, 15, 0.97)",
        sbai_glow: "rgba(255, 59, 48, 0.4)",
        sbai_surface_top: "#09090c",
        sbai_surface_bottom: "#5d0914",
        sbai_border: "rgba(255, 158, 105, 0.28)",
        sbai_text: "rgba(255, 239, 201, 0.98)",
        sbai_muted: "rgba(255, 184, 125, 0.88)",
        word_palette: ["#46dcff", "#ff3b30", "#ffd93d", "#ff3f76", "#8cecff", "#ff8e52"],
        fireworks_palette: ["#46dcff", "#ff3b30", "#ffd93d", "#ff3f76", "#8cecff"],
    }
}

fn sbai_state_copy(sbai: f64) -> (&'static str, &'static str) {
    if sbai < 0.5 {
        return ("还能忍", "AI 还在试探，你已经有点绷不住了。");
    }

    if sbai < 2.0 {
        return ("开始红温", "AI 每多自信一分，人就更想当场开骂。");
    }

    if sbai < 5.0 {
        return ("马上开骂", "AI 写得越笃定，人越接近发疯。");
    }

    ("彻底爆炸", "已经不是调试，是一场精神消耗战。")
}

fn render_line_chart(daily_counts: &[DailyCount]) -> String {
    if daily_counts.is_empty() {
        return r#"<div class="empty">没有聊天输入数据。</div>"#.to_owned();
    }

    let points = daily_counts
        .iter()
        .map(|point| {
            serde_json::json!({
                "label": point.label,
                "count": point.count,
            })
        })
        .collect::<Vec<_>>();
    let points_json = serde_json::to_string(&points)
        .unwrap()
        .replace("</", "<\\/");

    format!(
        r#"<div id="daily-chart" class="chart-host" data-points='{}' aria-label="daily profanity chart"></div><div id="daily-chart-fallback" class="chart-fallback">折线图加载失败。</div>"#,
        points_json
    )
}

fn render_word_cloud(word_counts: &[(String, i64)]) -> Result<String, ReportError> {
    if word_counts.is_empty() {
        return Ok(r#"<div class="empty">没有检测到脏话。</div>"#.to_owned());
    }

    let words = word_counts
        .iter()
        .take(32)
        .map(|(word, count)| {
            vec![
                serde_json::Value::String(word.clone()),
                serde_json::Value::from(*count),
            ]
        })
        .collect::<Vec<_>>();
    let words_json = serde_json::to_string(&words)
        .map_err(ReportError::WordCloudJson)?
        .replace("</", "<\\/");

Ok(format!(
        r#"<div class="cloud">
  <div id="word-cloud-viewport" class="cloud-viewport" aria-label="高频脏话词云，支持缩放和拖拽">
    <canvas id="word-cloud-scene" class="cloud-scene" data-words='{}'></canvas>
    <div id="word-cloud-fallback" class="cloud-fallback">3D 词云加载失败。</div>
  </div>
</div>"#,
        words_json
    ))
}

fn downloads_dir() -> Result<PathBuf, ReportError> {
    let home = std::env::var("HOME").map_err(ReportError::MissingHome)?;
    Ok(Path::new(&home).join("Downloads"))
}

fn report_filename() -> Result<String, ReportError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(ReportError::SystemTime)?
        .as_secs();
    Ok(format!("maleme-report-{timestamp}.html"))
}

fn day_label(day: i64) -> Result<String, ReportError> {
    let datetime = OffsetDateTime::from_unix_timestamp(day * 86_400).map_err(|source| {
        ReportError::InvalidTimestamp {
            value: day * 86_400,
            source,
        }
    })?;
    Ok(datetime.date().to_string())
}

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("HOME is not available")]
    MissingHome(#[source] std::env::VarError),
    #[error("failed to read system time")]
    SystemTime(#[source] std::time::SystemTimeError),
    #[error("invalid report timestamp `{value}`")]
    InvalidTimestamp {
        value: i64,
        #[source]
        source: time::error::ComponentRange,
    },
    #[error("failed to write {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to open browser for {path}")]
    OpenBrowser {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("browser open command failed for {path} with exit code {code}")]
    OpenBrowserStatus { path: PathBuf, code: i32 },
    #[error("failed to encode word cloud data")]
    WordCloudJson(#[source] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use crate::{AdapterKind, FuckDetector, UserMessage};

    use super::{build_report_data, render_report, report_headline, sbai_state_copy};

    #[test]
    fn builds_daily_counts_with_gaps() {
        let detector = FuckDetector::from_lexicon("fuck\n傻逼").unwrap();
        let messages = vec![
            UserMessage {
                adapter: AdapterKind::Codex,
                text: "fuck fuck".to_owned(),
                time: 0,
            },
            UserMessage {
                adapter: AdapterKind::Claude,
                text: "hello".to_owned(),
                time: 86_400_000,
            },
            UserMessage {
                adapter: AdapterKind::OpenCode,
                text: "傻逼".to_owned(),
                time: 172_800_000,
            },
        ];

        let data = build_report_data(&messages, 4, &detector).unwrap();

        assert_eq!(data.total_profanities, 3);
        assert_eq!(data.daily_counts.len(), 3);
        assert_eq!(data.daily_counts[0].count, 2);
        assert_eq!(data.daily_counts[1].count, 0);
        assert_eq!(data.daily_counts[2].count, 1);
        assert_eq!(data.word_counts[0], ("fuck".to_owned(), 2));
        assert_eq!(data.word_counts[1], ("傻逼".to_owned(), 1));
        assert_eq!(data.sbai, 7_500_000.0);
        assert_eq!(data.range_start, "1970-01-01");
        assert_eq!(data.range_end, "1970-01-03");
    }

    #[test]
    fn renders_expected_sections() {
        let detector = FuckDetector::from_lexicon("fuck").unwrap();
        let messages = vec![UserMessage {
            adapter: AdapterKind::Codex,
            text: "fuck".to_owned(),
            time: 0,
        }];

        let html = render_report(&messages, 2, &detector).unwrap();

        assert!(html.contains("SBAI 指数"));
        assert!(html.contains("你这一天骂了 AI 多少次！"));
        assert!(html.contains("你最喜欢这么骂！"));
        assert!(html.contains("fuck"));
        assert!(html.contains("心如止水"));
        assert!(html.contains("daily-chart"));
        assert!(html.contains("echarts.min.js"));
        assert!(html.contains("word-cloud-viewport"));
        assert!(html.contains("word-cloud-scene"));
        assert!(html.contains("getContext('2d')"));
        assert!(html.contains("sbai-alert"));
        assert!(html.contains("彻底爆炸"));
        assert!(html.contains("提交到 leaderboard 看看你有多能骂！"));
        assert!(html.contains("https://leaderboard.sbai.uk/submit"));
    }

    #[test]
    fn headline_changes_by_count() {
        assert!(report_headline("2026-01-01", "2026-01-03", 3).contains("心如止水"));
        assert!(report_headline("2026-01-01", "2026-01-03", 30).contains("心态也太平稳了叭"));
        assert!(report_headline("2026-01-01", "2026-01-03", 300).contains("喜报！！你从"));
    }

    #[test]
    fn sbai_copy_changes_by_level() {
        assert_eq!(sbai_state_copy(0.2).0, "还能忍");
        assert_eq!(sbai_state_copy(1.2).0, "开始红温");
        assert_eq!(sbai_state_copy(3.2).0, "马上开骂");
        assert_eq!(sbai_state_copy(8.8).0, "彻底爆炸");
    }
}
