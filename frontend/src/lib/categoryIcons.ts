// A fixed, curated set of glyphs a user can pick for a category (see
// `CategorySettingsDialog`/`/categories`'s icon picker) — replaces the old
// `sourceColor.ts` scheme of hashing a category's name into a colored dot.
// A hashed dot gave every category *a* color, but not one anyone chose,
// and (paired with the also-removed per-article "unread" dot) read as a
// permanent "something new here" indicator rather than a stable identity
// mark. An icon a user actually picks is both a clearer identity signal
// and, unlike a color, legible to color-blind users and in the muted/
// monochrome article-card chrome.
//
// This file holds only a small curated subset — the ones the picker shows
// by default and every category defaults to — not the whole icon pack.
// `CategoryIconPicker` also offers a search over the full vendored Lucide
// set (1848 icons, `lib/lucideIcons.ts`, generated from `@lucide/svelte` by
// `scripts/generate-lucide-icons.mjs`), lazily loaded per icon rather than
// bundled upfront, so picking any of the 1848 costs nothing until searched
// for. Still fully offline: every icon is a vendored local `.svelte` file,
// no remote icon-font/CDN fetch, same as this curated set. Every entry
// (curated and full-set alike) is drawn in the same 24×24 stroke style so
// the grid reads as one family.
import type { Component } from 'svelte';
import Folder from './icons/Folder.svelte';
import Newspaper from './icons/Newspaper.svelte';
import Book from './icons/Book.svelte';
import Globe from './icons/Globe.svelte';
import MapPin from './icons/MapPin.svelte';
import Compass from './icons/Compass.svelte';
import Plane from './icons/Plane.svelte';
import Train from './icons/Train.svelte';
import Car from './icons/Car.svelte';
import Bike from './icons/Bike.svelte';
import Anchor from './icons/Anchor.svelte';
import Home from './icons/Home.svelte';
import Building from './icons/Building.svelte';
import Briefcase from './icons/Briefcase.svelte';
import Code from './icons/Code.svelte';
import Cpu from './icons/Cpu.svelte';
import Database from './icons/Database.svelte';
import Camera from './icons/Camera.svelte';
import Music from './icons/Music.svelte';
import Headphones from './icons/Headphones.svelte';
import Mic from './icons/Mic.svelte';
import Film from './icons/Film.svelte';
import Gamepad from './icons/Gamepad.svelte';
import Palette from './icons/Palette.svelte';
import Pencil from './icons/Pencil.svelte';
import Coffee from './icons/Coffee.svelte';
import Utensils from './icons/Utensils.svelte';
import ShoppingBag from './icons/ShoppingBag.svelte';
import Shirt from './icons/Shirt.svelte';
import Gem from './icons/Gem.svelte';
import Gift from './icons/Gift.svelte';
import Calendar from './icons/Calendar.svelte';
import Users from './icons/Users.svelte';
import MessageCircle from './icons/MessageCircle.svelte';
import Heart from './icons/Heart.svelte';
import Leaf from './icons/Leaf.svelte';
import Paw from './icons/Paw.svelte';
import Dumbbell from './icons/Dumbbell.svelte';
import Rocket from './icons/Rocket.svelte';
import GraduationCap from './icons/GraduationCap.svelte';
import Lightbulb from './icons/Lightbulb.svelte';
import Banknote from './icons/Banknote.svelte';
import TrendingUp from './icons/TrendingUp.svelte';
import Scale from './icons/Scale.svelte';

/** The pack's generic, no-particular-identity entry — what every category
 *  defaults to (`db::schema`'s `V13` migration) and what an unrecognized
 *  or missing icon id falls back to via `categoryIcon()` below. */
export const DEFAULT_CATEGORY_ICON = 'folder';

export interface CategoryIconEntry {
	id: string;
	label: string;
	Icon: Component<{ size?: number }>;
}

export const CATEGORY_ICONS: CategoryIconEntry[] = [
	{ id: 'folder', label: 'General', Icon: Folder },
	{ id: 'newspaper', label: 'News', Icon: Newspaper },
	{ id: 'book', label: 'Reading', Icon: Book },
	{ id: 'globe', label: 'World', Icon: Globe },
	{ id: 'map-pin', label: 'Places', Icon: MapPin },
	{ id: 'compass', label: 'Outdoors', Icon: Compass },
	{ id: 'plane', label: 'Travel', Icon: Plane },
	{ id: 'train', label: 'Trains', Icon: Train },
	{ id: 'car', label: 'Cars', Icon: Car },
	{ id: 'bike', label: 'Cycling', Icon: Bike },
	{ id: 'anchor', label: 'Sea', Icon: Anchor },
	{ id: 'home', label: 'Home', Icon: Home },
	{ id: 'building', label: 'City', Icon: Building },
	{ id: 'briefcase', label: 'Work', Icon: Briefcase },
	{ id: 'code', label: 'Tech', Icon: Code },
	{ id: 'cpu', label: 'Hardware', Icon: Cpu },
	{ id: 'database', label: 'Data', Icon: Database },
	{ id: 'camera', label: 'Photography', Icon: Camera },
	{ id: 'music', label: 'Music', Icon: Music },
	{ id: 'headphones', label: 'Audio', Icon: Headphones },
	{ id: 'mic', label: 'Podcasts', Icon: Mic },
	{ id: 'film', label: 'Film & TV', Icon: Film },
	{ id: 'gamepad', label: 'Gaming', Icon: Gamepad },
	{ id: 'palette', label: 'Art', Icon: Palette },
	{ id: 'pencil', label: 'Writing', Icon: Pencil },
	{ id: 'coffee', label: 'Lifestyle', Icon: Coffee },
	{ id: 'utensils', label: 'Food', Icon: Utensils },
	{ id: 'shopping-bag', label: 'Shopping', Icon: ShoppingBag },
	{ id: 'shirt', label: 'Fashion', Icon: Shirt },
	{ id: 'gem', label: 'Luxury', Icon: Gem },
	{ id: 'gift', label: 'Gifts', Icon: Gift },
	{ id: 'calendar', label: 'Events', Icon: Calendar },
	{ id: 'users', label: 'Community', Icon: Users },
	{ id: 'message-circle', label: 'Social', Icon: MessageCircle },
	{ id: 'heart', label: 'Health', Icon: Heart },
	{ id: 'leaf', label: 'Nature', Icon: Leaf },
	{ id: 'paw', label: 'Pets', Icon: Paw },
	{ id: 'dumbbell', label: 'Fitness', Icon: Dumbbell },
	{ id: 'rocket', label: 'Science', Icon: Rocket },
	{ id: 'graduation-cap', label: 'Education', Icon: GraduationCap },
	{ id: 'lightbulb', label: 'Ideas', Icon: Lightbulb },
	{ id: 'banknote', label: 'Money', Icon: Banknote },
	{ id: 'trending-up', label: 'Markets', Icon: TrendingUp },
	{ id: 'scale', label: 'Law', Icon: Scale }
];

const ICONS_BY_ID = new Map(CATEGORY_ICONS.map((entry) => [entry.id, entry]));

/** Looks up an id in the curated pack only (no fallback) — lets a caller
 *  (`CategoryIcon`) tell "one of the curated 44, render synchronously"
 *  apart from "something from the wider Lucide set, load it
 *  (`DynamicIcon`/`lib/lucideIcons.ts`) instead". */
export function findCategoryIcon(id: string | null | undefined): CategoryIconEntry | undefined {
	return (id && ICONS_BY_ID.get(id)) || undefined;
}

/** Resolves an icon id (a category's `icon` field, possibly `null` for
 *  Uncategorized or absent on data from an older pack) to a pack entry,
 *  falling back to the generic folder glyph for anything unrecognized
 *  rather than rendering nothing. Only meaningful for the curated pack —
 *  callers that also need to handle the wider Lucide set should use
 *  `findCategoryIcon` + `DynamicIcon` instead. */
export function categoryIcon(id: string | null | undefined): CategoryIconEntry {
	return findCategoryIcon(id) || ICONS_BY_ID.get(DEFAULT_CATEGORY_ICON)!;
}
