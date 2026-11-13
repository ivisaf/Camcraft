# CamCraft documentation site

The source of the CamCraft documentation site (#688), built with
[Zola](https://www.getzola.org/) (a single binary; no Node toolchain).

```sh
cd site
zola serve          # preview at http://127.0.0.1:1111
zola build          # writes public/ (gitignored)
zola check          # build and check every link
```

- `content/` - the pages, as Markdown: `guide/` (user guide) and
  `developers/` (developer section). A page's place in its section is its
  `weight`.
- `templates/` - our own Tera templates; no third-party theme.
- `static/` - the stylesheet, copied as is.

The version shown on the site comes from `version.toml`, which the release
workflow writes from the workspace version before it builds. Without it the
site says it is a local preview. To preview the released look, write one:
`printf 'version = "0.16.0"\n' > version.toml` (gitignored).

The site is built and deployed by the `site` and `pages` jobs of
`.github/workflows/release.yml`, on a release tag only. Write content from
CamCraft's own code and docs: the clean-room protocol in `AGENTS.md`
applies here too.
