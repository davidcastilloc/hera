import { describe, expect, test } from 'vite-plus/test';

import type { Track } from '../../generated/typings';
import {
  filterTracks,
  removeRedundantFolders,
  stripAccents,
} from '../utils-library';

describe('stripAccents', () => {
  test('should remove all accentutated characters in a string by its non-accentuated version', () => {
    expect(
      stripAccents(
        'AbCdÀÁÂÃÄÅĄĀàáâãäåąāÒÓÔÕÕÖØòóôõöøÈÉÊËĘĒèéêëðęēÇĆČçćčÐÌÍÎÏĪìíîïīÙÚÛÜŪùúûüūÑŅñņŠŚšśŸÿýŽŹŻžźżŁĻłļŃŅńņàáãảạăằắẳẵặâầấẩẫậèéẻẽẹêềếểễệđùúủũụưừứửữựòóỏõọôồốổỗộơờớởỡợìíỉĩịäëïîüûñçýỳỹỵỷğışĞİŞĢģĶķ',
      ),
    ).toStrictEqual(
      'abcdaaaaaaaaaaaaaaaaoooooooooooooeeeeeeeeeeeeeccccccdiiiiiiiiiiuuuuuuuuuunnnnssssyyyzzzzzzllllnnnnaaaaaaaaaaaaaaaaaeeeeeeeeeeeduuuuuuuuuuuoooooooooooooooooiiiiiaeiiuuncyyyyygisgisggkk',
    );
  });
});

describe('removeRedundantFolders', () => {
  test('should return the same array if there are no duplicates or subpaths', () => {
    expect(
      removeRedundantFolders(['/users/me/music', '/users/me/videos']),
    ).toStrictEqual(['/users/me/music', '/users/me/videos']);
  });

  test('should remove duplicate entries', () => {
    expect(
      removeRedundantFolders([
        '/users/me/music',
        '/users/me/videos',
        '/users/me/music',
      ]),
    ).toStrictEqual(['/users/me/music', '/users/me/videos']);
  });

  test('should remove subpaths', () => {
    expect(
      removeRedundantFolders([
        '/tmp/data/music',
        '/users/me/music',
        '/users/me/music/archive',
        '/tmp/data',
      ]),
    ).toStrictEqual(['/users/me/music', '/tmp/data']);
  });
});

function makeTrack(overrides: Partial<Track>): Track {
  return {
    id: 't-1',
    path: '/music/sample.mp3',
    title: 'Sample',
    album: 'Booth',
    album_artist: 'DJ',
    artists: ['DJ'],
    genres: ['Salsa'],
    year: 2025,
    duration: 210,
    track_no: 1,
    track_of: 1,
    disk_no: 1,
    disk_of: 1,
    is_compilation: false,
    bpm: 94,
    camelot: '8A',
    ...overrides,
  };
}

describe('filterTracks DJ booth tokens', () => {
  const pool: Track[] = [
    makeTrack({ id: 'skip-track', title: 'Eres Tu', has_audio_skips: true }),
    makeTrack({
      id: 'diff-track',
      title: 'Salsa Alias',
      has_bpm_trust_diff: true,
    }),
    makeTrack({
      id: 'own-track',
      title: 'Mi Edicion',
      is_own_production: true,
    }),
    makeTrack({
      id: 'mix-track',
      title: 'Live Hora Loca Set',
      duration: 920,
      is_mix_over_10m: true,
    }),
  ];

  test('supports skip, diff, propia, and mix tokens', () => {
    expect(filterTracks(pool, 'skip').map((t) => t.id)).toEqual(['skip-track']);
    expect(filterTracks(pool, 'diff').map((t) => t.id)).toEqual(['diff-track']);
    expect(filterTracks(pool, 'propia').map((t) => t.id)).toEqual([
      'own-track',
    ]);
    expect(filterTracks(pool, 'mix').map((t) => t.id)).toEqual(['mix-track']);
  });
});
