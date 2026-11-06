# Runbook: deploy the landing page to Cloudflare Pages

The landing page is one static file, `site/index.html`, with no build step. The previous design stays
available at `site/index_old.html` (served as `/index_old.html`). Cloudflare Pages serves the
`site/` folder as is. Their download links point at assets of a GitHub Release (see
`.github/workflows/release.yml`), so a release must exist before the page goes live.

| Setting | Value |
|---|---|
| Repository | `duylongpro99/wuts-that-bash` |
| Production branch | `master` |
| Framework preset | None |
| Build command | empty (use `exit 0` if the form insists on a value) |
| Build output directory | `site` |
| Root directory | empty (repository root) |

## Before you start

- A Cloudflare account with access to **Workers & Pages**.
- The release the page links to exists and has its assets. Check with:

  ```sh
  gh release view v0.2.0 --repo duylongpro99/wuts-that-bash --json assets --jq '.assets[].name'
  ```

  Expect four `wib-<target>.tar.gz` files and four `.sha256` files.

Choose **one** of the two methods below. Cloudflare can't switch a Pages project between Git integration
and direct upload later; to change method, create a new project.

## Method A: Git integration (recommended)

Every push to `master` deploys production, and every other branch gets a preview URL.

1. In the Cloudflare dashboard, open **Workers & Pages → Create → Pages → Connect to Git**.
2. Authorize the Cloudflare GitHub app for `duylongpro99/wuts-that-bash` and select the repository.
3. Set the project name (for example `wib`). It becomes `<name>.pages.dev`; if the name is taken,
   Cloudflare adds a suffix.
4. Fill in the build settings from the table above, then click **Save and Deploy**.
5. Optional, to skip deploys for Rust-only commits: **Settings → Builds → Build watch paths**,
   include `site/*`.

## Method B: direct upload with Wrangler

Use this to deploy from your machine or from CI without connecting the repository.

```sh
npx wrangler login                     # or export CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID
npx wrangler pages project create wib --production-branch master   # first time only
npx wrangler pages deploy site --project-name wib --branch master
```

Run the deploy command from the repository root. Wrangler prints the deployment URL when it finishes.
Deploying with `--branch` set to anything other than `master` creates a preview deployment.

## Custom domain (optional)

1. Open the project, then **Custom domains → Set up a custom domain**.
2. Enter the domain. If its DNS is on Cloudflare, the record is created for you; otherwise add the
   `CNAME` record Cloudflare shows, pointing to `<name>.pages.dev`.
3. Wait for the status to become **Active**. The certificate is issued automatically.

## Verify

Replace `wib.pages.dev` with your URL.

```sh
curl -fsS -o /dev/null -w "%{http_code}\n" https://wib.pages.dev/                   # expect 200
curl -fsS https://wib.pages.dev/ | grep -o 'releases/download/v[0-9.]*' | sort -u    # expect the current tag only
curl -fsSL -o /dev/null -w "%{http_code}\n" \
  https://github.com/duylongpro99/wuts-that-bash/releases/download/v0.2.0/wib-aarch64-apple-darwin.tar.gz  # expect 200
```

Then open the page in a browser and check that the demo prompt animates, that switching platform
changes the commands and the download button, and that the copy button copies.

## Shipping a new version

The page names the tag in its links, so each release needs a page update.

1. Bump `version` in `Cargo.toml`, commit, then tag and push (the workflow rejects a tag that doesn't
   match `Cargo.toml`):

   ```sh
   git tag v0.2.0 && git push origin v0.2.0
   ```

2. Wait for the `release` workflow to finish and confirm the assets with `gh release view v0.2.0`.
3. Update the links and push:

   ```sh
   sed -i '' 's/v0\.1\.0/v0.2.0/g' site/index.html site/index_old.html   # GNU sed: sed -i 's/.../.../g'
   git commit -am "docs(site): link release v0.2.0" && git push origin master
   ```

   With Method A, the push deploys. With Method B, run the `wrangler pages deploy` command again.

Update the links only after the release exists, or the download buttons return 404.

## Rollback

- **Dashboard:** open the project, then **Deployments**, pick the last good deployment and choose
  **Rollback to this deployment**. This doesn't change the repository.
- **Git:** `git revert <commit>` and push to `master` (Method A), or check out the good commit and run
  `wrangler pages deploy` again (Method B).

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| Page shows a 404 at the root | Build output directory isn't `site`. Fix it in **Settings → Builds**, then retry the deployment. |
| Download buttons return 404 | The tag in `site/index.html` has no release, or the release has no assets yet. Check the workflow run. |
| No install commands appear under "Download and unpack" | The browser lacks CSS `:has()` support (very old browsers). Use the "All files" link. |
| A Rust-only commit triggered a deploy | Expected without build watch paths. Set them to `site/*` (Method A, step 5). |
