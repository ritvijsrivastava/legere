const DOT_OPACITIES = [100, 76, 56, 40, 88, 64];

/** Deterministic per-source dot color (category dots in the sidebar and
 *  on article cards) — since sources aren't a fixed enum, this hashes the
 *  name into a small rotation of accent tints rather than assigning a
 *  color per known category the way the design's mock data did. Same
 *  source name always gets the same dot color, wherever it's shown. */
export function sourceDotColor(name: string): string {
	let hash = 0;
	for (let i = 0; i < name.length; i++) {
		hash = (hash * 31 + name.charCodeAt(i)) | 0;
	}
	const opacity = DOT_OPACITIES[Math.abs(hash) % DOT_OPACITIES.length];
	return `color-mix(in srgb, var(--color-accent) ${opacity}%, var(--color-surface))`;
}
