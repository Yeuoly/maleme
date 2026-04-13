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

    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>maleme report</title>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/wordcloud2.js/1.2.3/wordcloud2.min.js"></script>
  <style>
    :root {{
      color-scheme: light;
      --bg: #eff8ff;
      --bg-accent: #fff4d6;
      --panel: rgba(255, 255, 255, 0.92);
      --panel-strong: #ffffff;
      --muted: #5e6b78;
      --text: #14212b;
      --line: #ff5a5f;
      --line-soft: rgba(255, 90, 95, 0.16);
      --accent: #00a6a6;
      --accent-warm: #ffb703;
      --accent-pink: #ff6f91;
      --border: #cfe1eb;
      --shadow: 0 18px 44px rgba(20, 33, 43, 0.1);
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background:
        radial-gradient(circle at top left, rgba(255, 183, 3, 0.28), transparent 24%),
        radial-gradient(circle at top right, rgba(255, 111, 145, 0.24), transparent 24%),
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
      grid-template-columns: minmax(0, 1.8fr) minmax(260px, 0.8fr);
      gap: 16px;
      align-items: stretch;
      min-height: 0;
    }}
    .panel {{
      background: var(--panel);
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
      background:
        linear-gradient(135deg, rgba(255, 255, 255, 0.98), rgba(255, 247, 229, 0.98));
    }}
    .hero-main::before {{
      content: "";
      position: absolute;
      inset: 0 auto auto 0;
      width: 180px;
      height: 180px;
      background: radial-gradient(circle, rgba(255, 183, 3, 0.35), transparent 70%);
      transform: translate(-30%, -30%);
    }}
    .hero-main::after {{
      content: "";
      position: absolute;
      inset: auto 0 0 auto;
      width: 200px;
      height: 200px;
      background: radial-gradient(circle, rgba(0, 166, 166, 0.22), transparent 70%);
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
      max-width: 13ch;
      text-wrap: balance;
      animation: headline-pop 2.4s ease-in-out infinite;
    }}
    .headline-count {{
      color: #e11919;
      text-shadow: 0 10px 22px rgba(225, 25, 25, 0.18);
    }}
    h2 {{
      margin: 0 0 14px;
      font-size: 20px;
      line-height: 1.2;
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
      background: rgba(255, 255, 255, 0.84);
      border: 1px solid rgba(207, 225, 235, 0.9);
      min-width: 132px;
    }}
    .meta-label {{
      color: var(--muted);
      font-size: 12px;
      margin-bottom: 6px;
    }}
    .meta-value {{
      font-size: 22px;
      font-weight: 700;
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
      display: flex;
      flex-direction: column;
      justify-content: space-between;
      min-height: 100%;
      background:
        linear-gradient(180deg, rgba(255, 255, 255, 0.98), rgba(229, 248, 248, 0.98));
    }}
    .sbai-label {{
      color: #0b7d7d;
      font-size: 13px;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      font-weight: 800;
    }}
    .sbai-value {{
      font-size: 92px;
      line-height: 0.88;
      font-weight: 800;
      margin: 8px 0 6px;
      color: #0b7d7d;
      font-variant-numeric: tabular-nums;
    }}
    .layout {{
      display: grid;
      grid-template-columns: minmax(0, 0.78fr) minmax(0, 1.22fr);
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
    .cloud {{
      position: relative;
      display: flex;
      align-items: center;
      justify-content: center;
      width: 100%;
      height: 100%;
      min-height: 0;
      overflow: hidden;
    }}
    .cloud-canvas {{
      width: 100%;
      height: 100%;
      min-height: 0;
      max-height: none;
    }}
    .cloud-fallback {{
      display: none;
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
    @media (max-width: 900px) {{
      body {{ height: auto; overflow: auto; }}
      .page {{ height: auto; padding: 16px; overflow: visible; }}
      .hero {{ grid-template-columns: 1fr; }}
      .layout {{ grid-template-columns: 1fr; }}
      h1 {{ font-size: 42px; max-width: 100%; }}
      .sbai-value {{ font-size: 68px; }}
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
        <div>
          <div class="sbai-label">SBAI 指数</div>
          <div class="sbai-value number-roll" data-target-number="{sbai:.2}" data-decimals="2">0.00</div>
        </div>
        <div class="subtle">每千万 tokens 的骂人次数</div>
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
      const canvas = document.getElementById("fireworks");
      const host = canvas.parentElement;
      const ctx = canvas.getContext("2d");
      const particles = [];
      const palette = ['#ff5a5f', '#00a6a6', '#ffb703', '#ff6f91', '#5bc0eb'];

      function resize() {{
        const rect = host.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
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
        const count = 54;
        for (let i = 0; i < count; i += 1) {{
          const angle = (Math.PI * 2 * i) / count;
          const speed = 1.9 + Math.random() * 3.8;
          particles.push({{
            x,
            y,
            vx: Math.cos(angle) * speed,
            vy: Math.sin(angle) * speed,
            life: 64 + Math.random() * 28,
            color: palette[i % palette.length],
            size: 2.4 + Math.random() * 3.6
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
      setInterval(() => {{
        burst();
        if (Math.random() > 0.45) {{
          burst();
        }}
      }}, 900);
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
            node.textContent = formatNumber(value, decimals);
          }});

          if (progress < 1) {{
            requestAnimationFrame(frame);
          }}
        }}

        requestAnimationFrame(frame);
      }}

      function renderWordCloud() {{
        const canvas = document.getElementById('word-cloud-canvas');
        if (!canvas || typeof WordCloud !== 'function') {{
          return;
        }}

        const words = JSON.parse(canvas.dataset.words || '[]');
        const rect = canvas.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
        canvas.width = rect.width * dpr;
        canvas.height = rect.height * dpr;

        WordCloud(canvas, {{
          list: words,
          gridSize: Math.max(6, Math.round(rect.width / 64)),
          weightFactor(size) {{
            return Math.max(22, size * 0.34);
          }},
          fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
          color(word, weight, fontSize, distance, theta) {{
            const palette = ['#ff6f61', '#00a6a6', '#ffb703', '#4cc9f0', '#48bfe3', '#f28482'];
            const index = Math.abs(Math.floor(theta * 10)) % palette.length;
            return palette[index];
          }},
          backgroundColor: 'rgba(255,255,255,0)',
          rotateRatio: 0.08,
          rotationSteps: 2,
          drawOutOfBound: false,
          shuffle: false,
          shape: 'circle',
          ellipticity: 0.72,
          shrinkToFit: true,
          minSize: 12,
          classes(word, weight) {{
            return 'cloud-word';
          }}
        }});
      }}

      animateNumbers();
      renderWordCloud();
      window.addEventListener('resize', renderWordCloud);
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

fn render_line_chart(daily_counts: &[DailyCount]) -> String {
    if daily_counts.is_empty() {
        return r#"<div class="empty">没有聊天输入数据。</div>"#.to_owned();
    }

    let width = 1080.0;
    let height = 220.0;
    let left = 56.0;
    let right = 20.0;
    let top = 20.0;
    let bottom = 44.0;
    let plot_width = width - left - right;
    let plot_height = height - top - bottom;
    let max_count = daily_counts
        .iter()
        .map(|point| point.count)
        .max()
        .unwrap_or(0)
        .max(1) as f64;
    let step_x = if daily_counts.len() == 1 {
        0.0
    } else {
        plot_width / (daily_counts.len() - 1) as f64
    };

    let mut path = String::new();
    let mut fill = format!("M {:.2} {:.2} ", left, top + plot_height);
    let mut circles = String::new();
    let mut x_labels = String::new();
    let label_stride = ((daily_counts.len() as f64) / 6.0).ceil().max(1.0) as usize;

    for (index, point) in daily_counts.iter().enumerate() {
        let x = left + step_x * index as f64;
        let y = top + plot_height - (point.count as f64 / max_count) * plot_height;

        if index == 0 {
            path.push_str(&format!("M {:.2} {:.2} ", x, y));
        } else {
            path.push_str(&format!("L {:.2} {:.2} ", x, y));
        }

        fill.push_str(&format!("L {:.2} {:.2} ", x, y));
        circles.push_str(&format!(
            r##"<circle class="chart-dot" style="animation-delay:{:.2}s" cx="{:.2}" cy="{:.2}" r="4" fill="#00a6a6" /><title>{}：你这一天骂了 AI {} 次！</title>"##,
            index as f64 * 0.04,
            x,
            y,
            escape_html(&point.label),
            point.count
        ));

        if index % label_stride == 0 || index + 1 == daily_counts.len() {
            x_labels.push_str(&format!(
                r##"<text x="{:.2}" y="{:.2}" text-anchor="middle" fill="#5e6b78" font-size="11">{}</text>"##,
                x,
                height - 12.0,
                escape_html(&point.label)
            ));
        }
    }

    fill.push_str(&format!(
        "L {:.2} {:.2} Z",
        left + plot_width,
        top + plot_height
    ));

    let mut y_grid = String::new();

    for step in 0..=4 {
        let ratio = step as f64 / 4.0;
        let y = top + plot_height - ratio * plot_height;
        let value = (max_count * ratio).round() as i64;
        y_grid.push_str(&format!(
            r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#d9e7ee" stroke-width="1" />
<text x="12" y="{:.2}" fill="#5e6b78" font-size="11">{}</text>"##,
            left,
            y,
            left + plot_width,
            y,
            y + 4.0,
            value
        ));
    }

    format!(
        r##"<svg viewBox="0 0 {width} {height}" width="100%" role="img" aria-label="daily profanity chart">
  <rect x="{left}" y="{top}" width="{plot_width}" height="{plot_height}" rx="8" fill="rgba(255,255,255,0.88)" />
  {y_grid}
  <path class="chart-fill" d="{fill}" fill="rgba(255,90,95,0.16)" />
  <path class="chart-line" d="{path}" fill="none" stroke="#ff5a5f" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" />
  {circles}
  {x_labels}
</svg>"##,
        width = width,
        height = height,
        left = left,
        top = top,
        plot_width = plot_width,
        plot_height = plot_height,
        y_grid = y_grid,
        fill = fill,
        path = path,
        circles = circles,
        x_labels = x_labels,
    )
}

fn render_word_cloud(word_counts: &[(String, i64)]) -> Result<String, ReportError> {
    if word_counts.is_empty() {
        return Ok(r#"<div class="empty">没有检测到脏话。</div>"#.to_owned());
    }

    let words = word_counts
        .iter()
        .take(42)
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
        r#"<div class="cloud"><canvas id="word-cloud-canvas" class="cloud-canvas" data-words='{}'></canvas><div class="cloud-fallback">词云加载失败。</div></div>"#,
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

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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

    use super::{build_report_data, render_report, report_headline};

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
        assert!(html.contains("提交到 leaderboard 看看你有多能骂！"));
        assert!(html.contains("https://leaderboard.sbai.uk/submit"));
    }

    #[test]
    fn headline_changes_by_count() {
        assert!(report_headline("2026-01-01", "2026-01-03", 3).contains("心如止水"));
        assert!(report_headline("2026-01-01", "2026-01-03", 30).contains("心态也太平稳了叭"));
        assert!(report_headline("2026-01-01", "2026-01-03", 300).contains("喜报！！你从"));
    }
}
