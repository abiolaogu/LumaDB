# LumaDB Presentations

## PowerPoint-Style Presentation Decks

This directory contains presentation materials in Markdown format that can be converted to PowerPoint using tools like Marp, Slidev, or pandoc.

---

## Available Presentations

| Presentation | Audience | Duration | File |
|-------------|----------|----------|------|
| Executive Overview | C-Suite, VPs | 20-30 min | [executive-overview.md](./executive-overview.md) |
| Technical Deep-Dive | Engineers, Architects | 45-60 min | [technical-deep-dive.md](./technical-deep-dive.md) |
| Sales Deck | Prospects, Customers | 30-45 min | [sales-deck.md](./sales-deck.md) |
| Training Overview | New Users, Developers | 60-90 min | [training-deck.md](./training-deck.md) |

---

## Converting to PowerPoint

### Using Marp CLI

```bash
# Install Marp CLI
npm install -g @marp-team/marp-cli

# Convert to PPTX
marp executive-overview.md --pptx -o executive-overview.pptx

# Convert to PDF
marp executive-overview.md --pdf -o executive-overview.pdf
```

### Using Pandoc

```bash
# Install pandoc
brew install pandoc  # macOS
apt install pandoc   # Linux

# Convert to PPTX
pandoc executive-overview.md -o executive-overview.pptx
```

### Using Slidev

```bash
# Install Slidev
npm install -g @slidev/cli

# Start presentation
slidev executive-overview.md

# Export to PDF
slidev export executive-overview.md
```

---

## Customization

Each presentation uses Marp-compatible syntax with:
- `---` for slide breaks
- Front matter for theme configuration
- Speaker notes in HTML comments

Feel free to customize colors, fonts, and branding for your organization.
