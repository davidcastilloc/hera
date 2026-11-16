import orderBy from 'lodash-es/orderBy';
import uniq from 'lodash-es/uniq';

import type { SortBy, SortOrder, Track } from '../generated/typings';
import type { Path } from '../types/museeks';

const SEARCH_BLOB_CACHE = new WeakMap<Track, string>();

function getTrackSearchBlob(track: Track): string {
  const cached = SEARCH_BLOB_CACHE.get(track);
  if (cached !== undefined) return cached;
  const blob = stripAccents(
    `${track.title} ${track.artists.join(' ')} ${track.album} ${track.genres.join(' ')} ${track.inferred_genre_family ?? ''}`,
  );
  SEARCH_BLOB_CACHE.set(track, blob);
  return blob;
}

const CAMELOT_TOKEN_RE = /^(1[0-2]|[1-9])[ab]$/i;
const BPM_RANGE_RE = /^(\d{2,3})-(\d{2,3})$/;
const BPM_EXACT_RE = /^\d{2,3}$/;

/**
 * Filter an array of tracks by string (cached O(1) blob + smart DJ tokens)
 */
export function filterTracks(tracks: Track[], search: string): Track[] {
  const trimmed = search.trim().toLowerCase();
  if (trimmed.length === 0) return tracks;

  const tokens = trimmed.split(/\s+/);
  return tracks.filter((track) => {
    for (const token of tokens) {
      if (CAMELOT_TOKEN_RE.test(token)) {
        if ((track.camelot ?? '').toLowerCase() !== token) return false;
      } else if (BPM_RANGE_RE.test(token)) {
        const [, minS, maxS] = token.match(BPM_RANGE_RE)!;
        const bpm = track.bpm ?? 0;
        if (bpm < Number(minS) || bpm > Number(maxS)) return false;
      } else if (BPM_EXACT_RE.test(token)) {
        const target = Number(token);
        const bpmMatch = Math.abs((track.bpm ?? 0) - target) <= 0.9;
        if (!bpmMatch && !getTrackSearchBlob(track).includes(token))
          return false;
      } else if (token === 'video') {
        if (track.is_video !== true) return false;
      } else if (token === 'transicion') {
        if (track.is_transition_candidate !== true) return false;
      } else if (token === 'fake320') {
        if (track.is_fake_320kbps !== true) return false;
      } else if (token === 'skip') {
        if (track.has_audio_skips !== true) return false;
      } else if (token === 'diff') {
        if (track.has_bpm_trust_diff !== true) return false;
      } else if (token === 'propia') {
        if (track.is_own_production !== true) return false;
      } else if (token === 'mix') {
        const isMix = track.is_mix_over_10m === true || track.duration > 600;
        if (!isMix) return false;
      } else {
        if (!getTrackSearchBlob(track).includes(stripAccents(token)))
          return false;
      }
    }
    return true;
  });
}

/**
 * Sort an array of tracks (alias to lodash.orderby)
 */
export function sortTracks(
  tracks: Track[],
  sortBy: SortConfig,
  sortOrder: SortOrder,
): Track[] {
  // The first column is sorted either asc or desc, but the rest is always asc
  const firstOrder = sortOrder === 'Asc' ? 'asc' : 'desc';
  return orderBy<Track>(tracks, sortBy, [firstOrder]);
}

/**
 * Strip accent from a string and lowercase them. From https://jsperf.com/strip-accents
 * Intentionally not idiomatic, it needs to be *fast*.
 */
export function stripAccents(str: string): string {
  let newStr = '';

  for (let i = 0; i < str.length; i++) {
    if (ACCENT_MAP.has(str[i])) {
      newStr += ACCENT_MAP.get(str[i]);
    } else {
      newStr += str[i];
    }
  }

  return newStr.toLowerCase();
}

const ACCENTS =
  'ÀÁÂÃÄÅĄĀàáâãäåąāÒÓÔÕÕÖØòóôõöøÈÉÊËĘĒèéêëðęēÇĆČçćčÐÌÍÎÏĪìíîïīÙÚÛÜŪùúûüūÑŅñņŠŚšśŸÿýŽŹŻžźżŁĻłļŃŅńņàáãảạăằắẳẵặâầấẩẫậèéẻẽẹêềếểễệđùúủũụưừứửữựòóỏõọôồốổỗộơờớởỡợìíỉĩịäëïîüûñçýỳỹỵỷğışĞİŞĢģĶķ';

const ACCENT_REPLACEMENTS =
  'AAAAAAAAaaaaaaaaOOOOOOOooooooEEEEEEeeeeeeeCCCcccDIIIIIiiiiiUUUUUuuuuuNNnnSSssYyyZZZzzzLLllNNnnaaaaaaaaaaaaaaaaaeeeeeeeeeeeduuuuuuuuuuuoooooooooooooooooiiiiiaeiiuuncyyyyygisGISGgKk';

const ACCENT_MAP = new Map();

for (let i = 0; i < ACCENTS.length; i++) {
  ACCENT_MAP.set(ACCENTS[i], ACCENT_REPLACEMENTS[i]);
}

/**
 * Given multiple paths as string, remove duplicates or child paths in case on parent exist in the array
 */
export const removeRedundantFolders = (paths: Array<string>): Array<string> => {
  return uniq(
    paths.filter((path) => {
      const isDuplicate = paths.some((otherPath) => {
        return path.startsWith(otherPath) && path !== otherPath;
      });

      return !isDuplicate;
    }),
  );
};

/** ----------------------------------------------------------------------------
 * Sort utilities
 * -------------------------------------------------------------------------- */

// For perforances reasons, otherwise _.orderBy will perform weird checks
// that are far more resource/time impactful
const ARTIST = (t: Track): string =>
  stripAccents(t.artists.toString().toLowerCase());
const GENRE = (t: Track): string =>
  stripAccents(t.genres.toString().toLowerCase());
const ALBUM = (t: Track): string => stripAccents(t.album.toLowerCase());
const TITLE = (t: Track): string => stripAccents(t.title.toLowerCase());

const CAMELOT_ORDER = (t: Track): number => {
  const c = (t.camelot ?? '').trim().toUpperCase();
  const match = /^(1[0-2]|[1-9])([AB])$/.exec(c);
  if (!match) return 9999;
  return Number(match[1]) * 10 + (match[2] === 'B' ? 1 : 0);
};

type TrackKeys = Path<Track>;
type IterateeFunction = (track: Track) => string | number;

type SortConfig = Array<TrackKeys | IterateeFunction>;

// Declarations
const SORT_ORDERS: Record<SortBy, SortConfig> = {
  Artist: [ARTIST, ALBUM, 'disk_no', 'track_no'],
  Title: [TITLE, ARTIST, ALBUM, 'disk_no', 'track_no'],
  Duration: ['duration', ARTIST, ALBUM, 'disk_no', 'track_no'],
  Album: [ALBUM, ARTIST, 'disk_no', 'track_no'],
  Genre: [GENRE, ARTIST, ALBUM, 'disk_no', 'track_no'],
  Bpm: ['bpm', ARTIST, ALBUM, 'disk_no', 'track_no'],
  Camelot: [CAMELOT_ORDER, ARTIST, ALBUM, 'disk_no', 'track_no'],
};

export function getSortOrder(sortBy: SortBy): SortConfig {
  return SORT_ORDERS[sortBy];
}
