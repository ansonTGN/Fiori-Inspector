# Fiori Inspector Studio

**Interactive Rust application for analyzing SAP Fiori / SAPUI5 applications without ChromeDriver.**

Author: **Angel A. Urbina**

---

## Overview

**Fiori Inspector Studio** is a professional Rust-based tool designed to analyze SAP Fiori and SAPUI5 web applications, extract their technical structure and help prepare robust automation workflows.

The project is focused on understanding Fiori screens in a way conceptually similar to how SAP GUI Scripting helps understand classic SAP GUI transactions, but adapted to the modern web architecture of SAP Fiori.

The tool analyzes:

* Rendered HTML DOM.
* SAPUI5 logical control structure.
* Buttons, inputs, links, tables, tabs and panels.
* UI5 identifiers.
* Candidate CSS selectors.
* Bindings and model paths when detectable.
* Possible OData traces.
* Automation actions.
* YAML workflows.

This version does **not** use ChromeDriver or GeckoDriver. Live analysis is based on **Chrome DevTools Protocol — CDP**.

---

## Why This Project Exists

SAP Fiori applications are dynamic web applications built on SAPUI5. They are not simple static HTML pages.

A Fiori screen may include:

* XML views.
* Controllers.
* Fragments.
* Dynamically generated controls.
* OData bindings.
* Smart controls.
* Tables.
* Value helps.
* Launchpad navigation.
* Runtime-generated IDs.

Because of this, automating Fiori only with mouse coordinates or simple HTML scraping is fragile.

**Fiori Inspector Studio** helps inspect the application structure before automation, making it easier to select stable technical references and design maintainable workflows.

---

## Main Features

### Apple-like Local Studio

The application includes a local browser interface with a clean, intuitive and Apple-inspired visual design.

Default local URL:

```text
http://127.0.0.1:7820
```

The interface includes:

* Main dashboard.
* Beginner-friendly help.
* Live Fiori analysis.
* Static HTML analysis.
* Control tree view.
* Action candidates.
* Selector recommendations.
* Workflow generation.
* Technical analysis panels.

---

### No ChromeDriver Required

This version removes the dependency on ChromeDriver.

Previous architecture:

```text
Rust → ChromeDriver → Chrome → SAP Fiori
```

Current architecture:

```text
Rust → Chrome DevTools Protocol → Chrome / Chromium → SAP Fiori
```

Chrome or Chromium is launched with a local remote debugging port, normally:

```text
http://127.0.0.1:9222
```

---

### Live SAP Fiori Analysis

The tool can connect to a running Chrome or Chromium session and analyze a real Fiori application.

It can extract:

* Page URL.
* Page title.
* DOM snapshot.
* UI5 controls when available.
* Visible actions.
* Candidate selectors.
* Input fields.
* Buttons.
* Tables.
* Links.
* Control texts.
* Accessibility attributes.
* Automation confidence.

---

### Static HTML Analysis

The application can also analyze saved HTML files.

This is useful for:

* Offline testing.
* Training.
* Technical documentation.
* Demonstrations.
* Evidence capture.
* Parser validation.

Static HTML mode is less powerful than live CDP mode because SAPUI5 runtime information may not be available in a saved HTML file.

---

### Action Discovery

The tool identifies potential automation targets such as:

* Buttons.
* Search fields.
* Input fields.
* Links.
* Tabs.
* Menu items.
* Table cells.
* Table rows.
* Toolbar actions.

Each action may include:

* Visible text.
* Selector candidate.
* HTML tag.
* UI5 ID when available.
* Role.
* Confidence score.
* Suggested interaction type.

---

### Workflow Support

The project supports YAML workflows to transform analysis into repeatable automation scenarios.

Example:

```yaml
name: "Fiori Material Search"

steps:
  - action: goto
    url: "https://fiori.example.com/sap/bc/ui2/flp"

  - action: wait_ui5
    timeout_secs: 60

  - action: snapshot
    save_as: "01_home_snapshot.json"

  - action: input
    selector: "input[id$='--searchField-I']"
    value: "4500001234"

  - action: press
    key: "Enter"

  - action: snapshot
    save_as: "02_after_search.json"
```

---

## Technical Stack

The project uses:

* **Rust**
* **Axum**
* **Tokio**
* **Chrome DevTools Protocol**
* **Reqwest**
* **Serde**
* **Serde JSON**
* **Serde YAML**
* **TOML**
* **Scraper**
* **HTML / CSS / JavaScript**

---

## Project Structure

```text
fiori-inspector-studio-rs/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── INSTALAR_Y_USAR.md
├── config/
│   └── local.toml.example
├── docs/
│   ├── arquitectura.md
│   └── arquitectura_interactiva.md
├── examples/
│   └── static_fiori_fragment.html
├── workflows/
│   └── fiori_sample_workflow.yaml
├── static/
│   ├── index.html
│   ├── styles.css
│   └── app.js
└── src/
    ├── browser.rs
    ├── config.rs
    ├── lib.rs
    ├── main.rs
    ├── models.rs
    ├── report.rs
    ├── static_html.rs
    ├── studio.rs
    ├── workflow.rs
    └── js/
        └── ui5_probe.js
```

---

## Requirements

Required:

* Rust stable.
* Cargo.
* Chrome or Chromium.
* Linux, Windows or macOS.

Recommended:

* Ubuntu 24.04 or later.
* Chromium or Google Chrome.
* Access to an authorized SAP Fiori environment.
* Permission to inspect or automate the target application.

---

## Installation

Clone the repository:

```bash
git clone https://github.com/ansonTGN/Fiori-Inspector.git
cd Fiori-Inspector
```

Build:

```bash
cargo build
```

Run:

```bash
cargo run
```

Open:

```text
http://127.0.0.1:7820
```

---

## Configuration

Copy the example configuration:

```bash
cp config/local.toml.example config/local.toml
```

Example:

```toml
[server]
bind = "127.0.0.1:7820"

[browser]
cdp_url = "http://127.0.0.1:9222"
chrome_binary = "google-chrome"
auto_launch = true
headless = false
accept_insecure_certs = true
window_width = 1600
window_height = 1000
user_data_dir = "./.browser-profile-cdp"
```

If you use Chromium instead of Google Chrome:

```toml
chrome_binary = "chromium"
```

or:

```toml
chrome_binary = "chromium-browser"
```

---

## Running the Studio

Simple mode:

```bash
cargo run
```

Explicit mode:

```bash
cargo run -- serve
```

Then open:

```text
http://127.0.0.1:7820
```

---

## Manual Chrome / Chromium CDP Start

If automatic launch is disabled, start Chrome manually:

```bash
google-chrome \
  --remote-debugging-port=9222 \
  --user-data-dir=./.browser-profile-cdp \
  --window-size=1600,1000
```

Or with Chromium:

```bash
chromium \
  --remote-debugging-port=9222 \
  --user-data-dir=./.browser-profile-cdp \
  --window-size=1600,1000
```

Check CDP status:

```bash
curl http://127.0.0.1:9222/json/version
```

Then run:

```bash
cargo run
```

---

## CLI Usage

### Analyze Static HTML

```bash
cargo run -- analyze-html \
  --input examples/static_fiori_fragment.html \
  --output runs/static_snapshot.json
```

### Capture a Live Fiori Snapshot

```bash
cargo run -- snapshot-cdp \
  --url "https://your-fiori-server/sap/bc/ui2/flp" \
  --output runs/fiori_snapshot.json
```

### Print Summary

```bash
cargo run -- summary \
  --input runs/fiori_snapshot.json
```

### Print UI5 / DOM Tree

```bash
cargo run -- tree \
  --input runs/fiori_snapshot.json \
  --max-depth 6
```

### List Automation Actions

```bash
cargo run -- actions \
  --input runs/fiori_snapshot.json
```

Filter actions:

```bash
cargo run -- actions \
  --input runs/fiori_snapshot.json \
  --contains material
```

---

## Recommended Professional Workflow

1. Start the studio.
2. Open the local web interface.
3. Launch or connect to Chrome through CDP.
4. Open the SAP Fiori application.
5. Log in manually if needed.
6. Navigate to the target Fiori screen.
7. Capture a snapshot.
8. Review controls, DOM structure and actions.
9. Identify stable selectors.
10. Generate a YAML workflow.
11. Test the workflow in a non-production environment.
12. Refine selectors and waits.
13. Document the automation scenario.

---

## Automation Strategy

Recommended priority:

1. Prefer official SAP APIs and OData services where possible.
2. Use Fiori Inspector Studio to understand the UI.
3. Use UI-level automation only when API-level automation is not available.
4. Avoid coordinate-based automation.
5. Prefer stable IDs, semantic labels and accessibility attributes.
6. Capture evidence before and after critical steps.
7. Store workflows in version control.
8. Validate everything in test environments before production use.

---

## Security Notes

This tool must only be used in authorized SAP environments.

Important rules:

* Do not commit passwords.
* Do not commit cookies.
* Do not commit SAP tokens.
* Do not commit `.env`.
* Do not commit `config/local.toml` if it contains internal URLs or credentials.
* Keep the local web server bound to `127.0.0.1`.
* Do not expose the studio to the internet.
* Use only with explicit authorization.
* Respect corporate cybersecurity policies.
* Do not use this tool to bypass access controls.

Recommended `.gitignore`:

```gitignore
/target/
.env
.env.*
config/local.toml
runs/
*.log
*.tmp
.browser-profile-cdp/
.vscode/
.idea/
.DS_Store
Thumbs.db
```

---

## Limitations

Known limitations:

* Static HTML mode cannot fully reconstruct SAPUI5 runtime state.
* Some SAPUI5 controls may use generated IDs.
* Highly customized Fiori applications may require additional heuristics.
* CDP requires a locally accessible Chrome or Chromium instance.
* Corporate SSO flows may require manual login.
* Not all OData endpoints can be inferred from the UI alone.
* This tool is not an official SAP product.

---

## Roadmap

Possible future improvements:

* Full workflow recorder.
* Workflow replay.
* OData metadata correlation.
* Screenshot annotation.
* HTML evidence reports.
* PDF export.
* Selector stability scoring.
* Visual UI5 tree graph.
* Playwright export.
* Selenium export.
* Rust automation code generation.
* Batch analysis of multiple Fiori apps.
* Integration with SAP Gateway/OData client.
* Authentication profile management.
* CI/CD workflow validation.

---

## Author

**Angel A. Urbina**

Industrial Engineer, Data Science and Cybersecurity professional, with experience in SAP, Rust, Python, industrial systems, automation and enterprise process analysis.

GitHub:

```text
https://github.com/ansonTGN
```

---

## License

Suggested license:

```text
MIT License
Copyright (c) Angel A. Urbina
```

---

## Disclaimer

This is an independent technical tool for SAP Fiori / SAPUI5 inspection, documentation and automation support.

It is not an official SAP product.

SAP, SAP Fiori, SAPUI5 and related names are trademarks or registered trademarks of SAP SE or its affiliates.

Use this software responsibly and only in environments where you have explicit permission.
