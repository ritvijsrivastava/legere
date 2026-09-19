#!/usr/bin/env node
// Regenerates `src/lib/icons/lucide/*.svelte` from the `@lucide/svelte`
// package's icon data. Run after bumping the `@lucide/svelte` dependency to
// pick up new/renamed icons: `node scripts/generate-lucide-icons.mjs`
// (from `frontend/`).
//
// Output is plain inline-SVG `.svelte` files matching the style of every
// other hand-vendored icon in `lib/icons/` (fixed 24x24 viewBox,
// `currentColor` stroke, no dependency on `@lucide/svelte`'s own `Icon.svelte`
// wrapper/runtime) so they vendor and lazy-load (`lib/lucideIcons.ts`) the
// same way as the rest of the app's icon pack.
import { readFileSync, readdirSync, writeFileSync, mkdirSync, existsSync, rmSync } from 'node:fs';
import { join } from 'node:path';

const SRC_DIR = 'node_modules/@lucide/svelte/dist/icons';
const OUT_DIR = 'src/lib/icons/lucide';

if (existsSync(OUT_DIR)) rmSync(OUT_DIR, { recursive: true });
mkdirSync(OUT_DIR, { recursive: true });

function attrsToHtml(attrs) {
	return Object.entries(attrs)
		.map(([k, v]) => `${k}="${String(v).replace(/"/g, '&quot;')}"`)
		.join(' ');
}

const files = readdirSync(SRC_DIR).filter((f) => f.endsWith('.svelte'));
let count = 0;

for (const file of files) {
	const id = file.replace(/\.svelte$/, '');
	const src = readFileSync(join(SRC_DIR, file), 'utf8');
	const match = src.match(/const iconData = (\{[\s\S]*?\});/);
	if (!match) {
		console.error(`skip ${file}: no iconData literal found`);
		continue;
	}
	// lucide-svelte emits iconData as a JSON-compatible object literal.
	const iconData = JSON.parse(match[1]);
	const nodes = iconData.node
		.map(([tag, attrs]) => `\t<${tag} ${attrsToHtml(attrs)}></${tag}>`)
		.join('\n');
	const out = `<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
${nodes}
</svg>
`;
	writeFileSync(join(OUT_DIR, `${id}.svelte`), out);
	count++;
}

console.log(`Generated ${count} icons into ${OUT_DIR}`);
