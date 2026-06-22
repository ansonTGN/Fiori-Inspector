# Fiori Inspector Studio

**Interactive Rust application for analyzing SAP Fiori / SAPUI5 applications and preparing robust automation workflows.**

Author: **Angel A. Urbina**

---

## 1. Overview

**Fiori Inspector Studio** is a professional Rust-based tool designed to inspect, understand and prepare automation strategies for **SAP Fiori** and **SAPUI5** applications.

The main objective of this project is to provide an intuitive, clear and visually refined analysis environment that helps technical users discover the internal structure of a Fiori application in a way that is conceptually similar to how SAP GUI Scripting allows interaction with classic SAP GUI transactions.

Instead of relying only on fragile visual clicks or screen coordinates, this tool analyzes the application through several complementary layers:

* The rendered HTML DOM.
* The logical SAPUI5 control tree.
* UI5 control identifiers.
* Candidate CSS selectors.
* Visible user interface actions.
* Inputs, buttons, links, tables and panels.
* Bindings and model paths when available.
* OData endpoints and technical traces when detectable.
* YAML workflows for reproducible automation.

The application includes a local web interface with an Apple-inspired design focused on clarity, simplicity and usability.

---

## 2. Purpose

SAP Fiori applications are modern web applications built with SAPUI5. They are dynamic, component-based and frequently backed by OData services.

Traditional automation approaches based only on mouse coordinates or plain HTML scraping are often unreliable because SAPUI5 dynamically generates controls, IDs, tables, fragments and views.

This project aims to solve that problem by creating a structured analysis layer capable of helping the user answer questions such as:

* What controls exist on this Fiori screen?
* Which buttons, fields and tables can be automated?
* Which selectors are more stable?
* Which UI5 controls correspond to visible HTML elements?
* Which fields are bound to models or OData paths?
* What actions can be converted into repeatable automation workflows?
* How can I document and understand a Fiori application before automating it?

---

## 3. Main Features

### Interactive Studio

The application includes a local web studio accessible from the browser.

Default URL:

```text
http://127.0.0.1:7820
```

The interface provides:

* A clean Apple-like visual design.
* Main dashboard.
* Visual help section.
* Fiori live session capture.
* Static HTML analysis.
* UI5 control tree viewer.
* Action candidates panel.
* Selector recommendations.
* Bindings and model information.
* OData endpoint discovery.
* Workflow generation support.
* Beginner-friendly help integrated into the homepage.

---

### Live Fiori / SAPUI5 Inspection

Using WebDriver, the tool can open and inspect a live SAP Fiori application.

It can capture:

* DOM structure.
* UI5 controls.
* Control IDs.
* Control types.
* Texts and labels.
* Parent-child relationships.
* Visibility state.
* Tables and rows.
* Inputs and buttons.
* Candidate selectors.
* Automation confidence score.

This is the recommended mode for analyzing real SAP Fiori applications.

---

### Static HTML Analysis

The tool can also analyze saved HTML files.

This mode is useful for:

* Offline analysis.
* Documentation.
* Training.
* Evidence capture.
* Sharing simplified examples.
* Testing parser behavior without accessing a real SAP system.

Static HTML analysis is less powerful than live browser inspection because SAPUI5 runtime information may not be available once the page is saved as plain HTML.

---

### UI5 Logical Control Tree

SAPUI5 applications are not only HTML pages. They are built from UI5 controls such as:

* `sap.m.Button`
* `sap.m.Input`
* `sap.m.Table`
* `sap.m.Page`
* `sap.m.Panel`
* `sap.ui.comp.smarttable.SmartTable`
* `sap.ui.layout.form.SimpleForm`

Fiori Inspector Studio attempts to extract and represent this logical control structure so that automation can be based on meaningful application objects rather than unstable screen coordinates.

---

### Action Discovery

The tool identifies candidate UI actions such as:

* Buttons.
* Input fields.
* Search fields.
* Links.
* Tabs.
* Table rows.
* Table cells.
* Menu items.
* Toolbar actions.

Each detected action may include:

* Visible text.
* Control type.
* DOM selector.
* UI5 ID.
* Accessibility role.
* Automation confidence.
* Suggested interaction type.

---

### Selector Recommendations

The tool generates selector candidates that may be useful for automation.

Examples:

```css
#application-ZMM-display-component---Main--searchButton
[id$='--searchButton']
button[aria-label='Search']
.sapMBtn
```

The objective is to help the user choose selectors that are:

* Stable.
* Readable.
* Easy to maintain.
* Less dependent on generated DOM details.
* Suitable for WebDriver-based automation.

---

### Workflow Support

The project includes support for YAML-based workflows.

Example:

```yaml
name: "Fiori Material Search"

steps:
  - action: goto
    url: "https://fiori.example.com/sap/bc/ui2/flp"

  - action: wait_ui5
    timeout_secs: 90

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

This makes it possible to transform exploratory analysis into reproducible automation scenarios.

---

## 4. Architecture

The project follows a modular Rust architecture.

```text
fiori-inspector-studio-rs/
├── Cargo.toml
├── README.md
├── config/
│   └── local.toml.example
├── docs/
│   └── arquitectura.md
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

## 5. Technical Stack

The project is built with:

* **Rust** as the main programming language.
* **Axum** for the local web server.
* **Tokio** for asynchronous execution.
* **Thirtyfour** for WebDriver integration.
* **Scraper** for static HTML parsing.
* **Serde / Serde JSON / Serde YAML** for structured data.
* **TOML** for configuration.
* **HTML, CSS and JavaScript** for the local interactive studio.

---

## 6. Requirements

### Required

* Rust stable toolchain.
* Cargo.
* A modern browser.
* Linux, Windows or macOS.

### Recommended for live Fiori analysis

* Google Chrome or Chromium.
* ChromeDriver.
* Access to a SAP Fiori / SAPUI5 application.
* Valid SAP user authorization.
* Permission to analyze or automate the target environment.

---

## 7. Installation

Clone the repository:

```bash
git clone https://github.com/ansonTGN/Fiori-Inspector.git
cd Fiori-Inspector
```

Build the application:

```bash
cargo build
```

Run tests:

```bash
cargo test
```

Format code:

```bash
cargo fmt
```

Run static checks:

```bash
cargo clippy --all-targets -- -D warnings
```

---

## 8. Configuration

Copy the example configuration:

```bash
cp config/local.toml.example config/local.toml
```

Example configuration:

```toml
[server]
bind = "127.0.0.1:7820"

[browser]
webdriver_url = "http://127.0.0.1:9515"
browser = "chrome"
headless = false
accept_insecure_certs = true
window_width = 1440
window_height = 1000
```

Do not commit local configuration files containing sensitive information.

Recommended `.gitignore` entries:

```gitignore
/target/
.env
.env.*
config/local.toml
runs/
*.log
```

---

## 9. Usage

### Start the Interactive Studio

```bash
cargo run
```

or explicitly:

```bash
cargo run -- serve
```

Open:

```text
http://127.0.0.1:7820
```

---

### Start ChromeDriver

For live browser inspection, start ChromeDriver in a separate terminal:

```bash
chromedriver --port=9515
```

Check status:

```bash
curl http://127.0.0.1:9515/status
```

Then start the studio:

```bash
cargo run
```

Open the web interface and enter the SAP Fiori URL to inspect.

---

### Capture a Live Fiori Snapshot from CLI

```bash
cargo run -- snapshot-browser \
  --url "https://your-fiori-server/sap/bc/ui2/flp" \
  --output runs/fiori_snapshot.json
```

---

### Analyze Static HTML

```bash
cargo run -- analyze-html \
  --input examples/static_fiori_fragment.html \
  --output runs/static_snapshot.json
```

---

### Print Snapshot Summary

```bash
cargo run -- summary \
  --input runs/fiori_snapshot.json
```

---

### Print UI5 Control Tree

```bash
cargo run -- tree \
  --input runs/fiori_snapshot.json \
  --max-depth 6
```

---

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

## 10. Recommended Workflow

A professional analysis workflow may look like this:

1. Start ChromeDriver.
2. Start Fiori Inspector Studio.
3. Open the local web interface.
4. Enter the target SAP Fiori URL.
5. Log in manually if required.
6. Capture the current screen.
7. Review detected controls.
8. Review candidate actions.
9. Inspect selectors.
10. Identify stable UI5 IDs.
11. Detect bindings and OData traces.
12. Generate an initial workflow.
13. Test the workflow.
14. Refine selectors and waits.
15. Document the automation scenario.

---

## 11. Automation Strategy

The recommended automation strategy is:

1. Prefer official APIs and OData services when possible.
2. Use UI5 control inspection to understand the application.
3. Use WebDriver automation only when API-level automation is not available.
4. Avoid coordinate-based automation.
5. Avoid brittle selectors based only on generated CSS classes.
6. Prefer stable IDs, semantic labels and accessibility attributes.
7. Capture snapshots before and after important actions.
8. Store workflows as version-controlled YAML files.
9. Validate automations in non-production environments first.

---

## 12. Security and Compliance

This tool is intended for legitimate analysis, testing, documentation and automation in authorized SAP environments.

Important recommendations:

* Use only with explicit authorization.
* Do not store SAP passwords in the repository.
* Do not commit cookies, tokens or session files.
* Do not expose the local studio publicly.
* Keep the server bound to `127.0.0.1` unless there is a controlled reason to do otherwise.
* Use test or development environments whenever possible.
* Follow corporate cybersecurity and SAP governance policies.
* Do not use this tool to bypass access controls.
* Respect user permissions and audit requirements.

---

## 13. Limitations

Fiori Inspector Studio is not a replacement for official SAP APIs, SAP Gateway, SAP Business Application Studio or SAP Fiori Tools.

Known limitations:

* Static HTML mode cannot fully reconstruct the SAPUI5 runtime.
* Dynamically generated IDs may change between sessions.
* Some UI5 controls may not expose all metadata.
* Applications using custom controls may require additional extractor logic.
* Authentication flows depend on the corporate SAP landscape.
* WebDriver automation may require specific browser and driver versions.
* Highly customized Fiori applications may need project-specific heuristics.

---

## 14. Roadmap

Possible future improvements:

* OData metadata correlation.
* Automatic mapping between UI5 bindings and OData entities.
* Visual tree graph.
* Screenshot annotation.
* Stable selector scoring engine.
* Workflow recorder.
* Workflow replay.
* Export to Playwright.
* Export to Selenium-compatible scripts.
* Export to Rust automation modules.
* Integration with SAP Gateway client.
* Authentication profile management.
* Test evidence reports in HTML/PDF.
* Role-based inspection templates.
* Support for batch analysis of multiple Fiori apps.
* CI/CD validation of automation workflows.

---

## 15. Project Philosophy

The project follows several design principles:

* Make complex SAPUI5 structures understandable.
* Favor clarity over hidden magic.
* Prefer stable technical identifiers over fragile visual automation.
* Keep the user interface simple and elegant.
* Help non-experts understand what is happening.
* Create reusable automation assets.
* Support professional documentation and auditability.
* Use Rust for safety, performance and maintainability.

---

## 16. Example Use Cases

Possible use cases include:

* Understanding a new SAP Fiori application.
* Preparing automation for repetitive business processes.
* Creating technical documentation of Fiori screens.
* Discovering UI5 control structures.
* Identifying OData-related bindings.
* Supporting QA and regression testing.
* Building internal automation tools.
* Training technical teams in SAPUI5 inspection.
* Migrating manual SAP interactions to structured workflows.
* Comparing Fiori behavior across environments.

---

## 17. Development Commands

Build:

```bash
cargo build
```

Run:

```bash
cargo run
```

Run with explicit command:

```bash
cargo run -- serve
```

Run tests:

```bash
cargo test
```

Format:

```bash
cargo fmt
```

Lint:

```bash
cargo clippy --all-targets -- -D warnings
```

Generate documentation:

```bash
cargo doc --open
```

---

## 18. Repository

GitHub repository:

```text
https://github.com/ansonTGN/Fiori-Inspector
```

---

## 19. Author

**Angel A. Urbina**

Industrial Engineer, Data Science and Cybersecurity professional, with experience in SAP, Rust, Python, industrial systems, automation and enterprise process analysis.

GitHub:

```text
https://github.com/ansonTGN
```

---

## 20. License

This project can be distributed under the MIT License unless another license is explicitly defined by the author.

Suggested license:

```text
MIT License
Copyright (c) Angel A. Urbina
```

---

## 21. Disclaimer

This project is an independent technical tool for inspection, documentation and automation support.

It is not an official SAP product.

SAP, SAP Fiori, SAPUI5 and related names are trademarks or registered trademarks of SAP SE or its affiliates.

Use this software responsibly and only in environments where you have permission to inspect, test or automate applications.
