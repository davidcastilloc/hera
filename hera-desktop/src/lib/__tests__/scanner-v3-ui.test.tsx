import { renderToStaticMarkup } from 'react-dom/server';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => {
  if (
    typeof (globalThis as unknown as Record<string, unknown>).window ===
    'undefined'
  ) {
    (globalThis as unknown as Record<string, unknown>).window = globalThis;
  }
  return {
    invokeMock: vi.fn<(...args: unknown[]) => Promise<unknown>>(),
  };
});

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
  invoke: invokeMock,
}));

import CrateRulesBanner, {
  extractLedgerTxIds,
} from '../../components/CrateRulesBanner';
import FolderCrateTree from '../../components/FolderCrateTree';
import PreGigAuditPopover, {
  formatPreGigIssueKind,
} from '../../components/PreGigAuditPopover';
import { ScannerStatusPill } from '../../components/ScannerStatusPill';
import TrackRow from '../../components/TrackRow';
import TransitionBridgeStepsCard from '../../components/transition/TransitionBridgeStepsCard';
import {
  classifyWaveformFormat,
  computeRgbBandColor,
  decodeWaveformPeaks,
} from '../../components/WaveformPeaksCanvas';
import type { Track } from '../../generated/typings';
import {
  deriveFolderCratesFromTracks,
  type FolderCrateSummary,
  formatFolderCrateSubtitle,
  fromFolderPlaylistId,
  isFolderPlaylistId,
  toFolderPlaylistId,
} from '../omkf-bridge';
import {
  auditPreGigCrate,
  buildLocalPreGigAuditFallback,
  cancelBackgroundScan,
  setExternalBoothMode,
  useScannerStore,
} from '../scanner-v3';

function makeBoothTrack(overrides: Partial<Track>): Track {
  return {
    id: 'track-base',
    path: '/music/booth.mp3',
    title: 'Booth Track',
    album: 'Booth Album',
    album_artist: 'DaviDJ',
    artists: ['DaviDJ'],
    genres: ['Merengueton'],
    year: 2026,
    duration: 210,
    track_no: 1,
    track_of: 1,
    disk_no: 1,
    disk_of: 1,
    is_compilation: false,
    bpm: 124,
    camelot: '8A',
    ...overrides,
  };
}

describe('WaveformPeaksCanvas RGB 768 B & 512 B fallback helpers', () => {
  it('detects 768 B tricolor RGB vs 512 B mono peaks and decodes base64', () => {
    const rgb = new Uint8Array(768);
    rgb[0] = 240;
    rgb[1] = 20;
    rgb[2] = 10;
    let bin = '';
    for (const byte of rgb) bin += String.fromCharCode(byte);
    const b64 = btoa(bin);

    const decoded = decodeWaveformPeaks(null, b64);
    expect(decoded).not.toBeNull();
    expect(decoded?.length).toBe(768);
    expect(classifyWaveformFormat(decoded)).toBe('rgb-768');

    const mono512 = new Uint8Array(512).fill(64);
    expect(classifyWaveformFormat(mono512)).toBe('mono-512');
    expect(decodeWaveformPeaks(null, '   ')).toBeNull();
    expect(classifyWaveformFormat(null)).toBe('none');
  });

  it('blends Sub/Mid/Air channels into distinct dark booth RGBA colors', () => {
    const subDominant = computeRgbBandColor(255, 0, 0, 0.9);
    const midDominant = computeRgbBandColor(0, 255, 0, 0.9);
    const airDominant = computeRgbBandColor(0, 0, 255, 0.9);

    expect(subDominant).toBe('rgba(239, 68, 68, 0.9)');
    expect(midDominant).toBe('rgba(20, 184, 166, 0.9)');
    expect(airDominant).toBe('rgba(56, 189, 248, 0.9)');
  });
});

describe('PreGigAuditPopover, ScannerStatusPill & Track/Transition badges', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('audits denon_cd_mp3 sample rates, video incompatibility, and CD skips cleanly', async () => {
    const tracks = [
      makeBoothTrack({ id: 'ok', sample_rate_hz: 44100 }),
      makeBoothTrack({
        id: 'vid',
        title: 'Video Edit',
        is_video: true,
        sample_rate_hz: 48000,
      }),
      makeBoothTrack({
        id: 'glitch',
        title: 'CD Skip',
        has_audio_skips: true,
        sample_rate_hz: 44100,
      }),
    ];

    const denonReport = buildLocalPreGigAuditFallback('denon_cd_mp3', tracks);
    expect(denonReport.total_tracks).toBe(3);
    expect(denonReport.ready_count).toBe(1);
    expect(denonReport.blocker_count).toBe(3);
    expect(denonReport.passed).toBe(false);
    expect(formatPreGigIssueKind('sample_rate_incompatible')).toBe('44.1 kHz');

    invokeMock.mockResolvedValueOnce(denonReport);
    const viaIpc = await auditPreGigCrate('denon_cd_mp3', ['ok', 'vid']);
    expect(invokeMock).toHaveBeenCalledWith(
      'plugin:hera-dj|scanner_audit_pre_gig_crate',
      { targetProfile: 'denon_cd_mp3', trackIds: ['ok', 'vid'] },
    );
    expect(viaIpc.total_tracks).toBe(3);

    const popoverHtml = renderToStaticMarkup(
      <PreGigAuditPopover tracks={tracks} crateName="Salsa_VIP" />,
    );
    expect(popoverHtml).toContain('Check Pre-Toque');
  });

  it('ScannerStatusPill exposes Cancelar and Cabina Externa controls', async () => {
    const html = renderToStaticMarkup(
      <ScannerStatusPill
        previewState={{
          phase: 'analyzing',
          fastDone: 10,
          fullDone: 4,
          total: 100,
          tracksPerSec: 12,
          etaSec: 8,
          governorActive: false,
          externalBoothMode: false,
          pendingFrozenCount: 0,
        }}
      />,
    );
    expect(html).toContain('Cancelar');
    expect(html).toContain('Cabina Externa');
    expect(html).toContain('Analizando · 10/100');

    invokeMock.mockResolvedValue(true);
    await cancelBackgroundScan();
    expect(invokeMock).toHaveBeenCalledWith(
      'plugin:hera-dj|scanner_cancel_scan',
    );

    await setExternalBoothMode(true);
    expect(invokeMock).toHaveBeenCalledWith(
      'plugin:hera-dj|scanner_set_external_booth_mode',
      { enabled: true },
    );
    expect(useScannerStore.getState().externalBoothMode).toBe(true);
  });

  it('TrackRow & TransitionBridgeStepsCard render Vocal Clash, Glitch & Variable-BPM badges', () => {
    const flagged = makeBoothTrack({
      id: 'flagged-1',
      artist: 'Orquesta En Vivo',
      title: 'Tema En Vivo',
      has_audio_skips: true,
      is_variable_bpm: true,
      entry_bpm: 92.4,
      exit_bpm: 98.6,
      has_intro_vocals: true,
      has_outro_vocals: true,
      is_own_production: true,
      has_bpm_trust_diff: true,
    } as Partial<Track>);

    const rowHtml = renderToStaticMarkup(
      <TrackRow
        track={flagged}
        index={0}
        selected={false}
        onTrackSelect={() => {}}
        onContextMenu={() => {}}
        onPlaybackStart={() => {}}
      />,
    );
    expect(rowHtml).toContain('Salto');
    expect(rowHtml).toContain('BPM~');
    expect(rowHtml).toContain('Voz I/O');
    expect(rowHtml).toContain('Propia');
    expect(rowHtml).toContain('Diff BPM');

    const bridgeHtml = renderToStaticMarkup(
      <TransitionBridgeStepsCard
        steps={[
          {
            step_number: 1,
            track_id: 'flagged-1',
            artist: 'Orquesta En Vivo',
            title: 'Tema En Vivo',
            bpm: 95,
            camelot: '8A',
            owner_dj: 'DaviDJ',
            hera_seal: 'seal',
            cue_in_seconds: 12,
            cue_out_seconds: 180,
            synergy_score: 0.92,
            transition_tip: '',
          },
        ]}
        activeAuditionId={null}
        findMatchingLocalTrack={() => flagged}
        onAuditionStep={() => {}}
      />,
    );
    expect(bridgeHtml).toContain('Choque vocal');
    expect(bridgeHtml).toContain('Salto CD');
    expect(bridgeHtml).toContain('BPM~');
  });
});

describe('OMKF Milestone O5 UI components & helpers', () => {
  const sampleCrate: FolderCrateSummary = {
    crate_id: 'crate-salsa-baul',
    folder_path: '/music/Salsa Baul',
    display_name: 'Salsa Baul',
    tree_seal_64: '9a8b7c6d',
    track_count: 42,
    video_count: 6,
    full_count: 42,
    bpm_min: 86,
    bpm_max: 98,
    dominant_camelot: '8A',
    dominant_genre: 'SalsaBaulRomantica',
    era_span: '1992-1998',
    lufs_avg: -11.2,
    has_custom_rules: true,
    updated_at_unix_ms: 1700000000000,
  };

  it('formats folder playlist IDs, subtitles, and ledger tx IDs cleanly', () => {
    expect(toFolderPlaylistId('abc')).toBe('folder--abc');
    expect(isFolderPlaylistId('folder--abc')).toBe(true);
    expect(fromFolderPlaylistId('folder--abc')).toBe('abc');
    expect(formatFolderCrateSubtitle(sampleCrate)).toBe(
      '86–98 BPM · 8A · 100% FULL',
    );
    expect(
      extractLedgerTxIds([
        '- 2026-11-16T10:00Z [agent:organizer] tx:#org_01',
        '- 2026-11-16T10:05Z tx:#org_02',
        '- tx:org_03 | applied 4 moves | 2026-11-16T10:10Z',
      ]),
    ).toEqual(['org_01', 'org_02', 'org_03']);
  });

  it('renders CrateRulesBanner and FolderCrateTree cleanly in SSR', () => {
    const bannerHtml = renderToStaticMarkup(
      <CrateRulesBanner
        crate={sampleCrate}
        rulesMarkdown="Mantener flujo romántico sin mezclar con perreo actual."
        ledgerEntries={['- 2026-11-16T10:00Z [agent:organizer] tx:#tx_99']}
      />,
    );
    expect(bannerHtml).toContain('OMKF .hera');
    expect(bannerHtml).toContain('86–98 BPM · 8A · 100% FULL');
    expect(bannerHtml).toContain('Reglas de Cabina');
    expect(bannerHtml).toContain('Organizar con IA');

    const treeHtml = renderToStaticMarkup(
      <FolderCrateTree
        crates={[sampleCrate]}
        selectedCrateId="crate-salsa-baul"
        onSelectCrate={() => {}}
        collapsed={false}
        onToggleCollapsed={() => {}}
        totalTrackCount={42}
      />,
    );
    expect(treeHtml).toContain('Carpetas (.hera)');
    expect(treeHtml).toContain('Salsa Baul');
    expect(treeHtml).toContain('86–98 BPM · 8A · 100% FULL');
  });

  it('deriveFolderCratesFromTracks synthesizes folder crates from track paths when DB table is empty', () => {
    const derived = deriveFolderCratesFromTracks(
      [
        {
          id: 't1',
          path: 'D:/Music/01_Afro_House/Move.mp3',
          title: 'Move',
          album: 'Single',
          album_artist: 'Adam Port',
          artists: ['Adam Port'],
          genres: ['Afro House'],
          year: 2025,
          duration: 210,
          track_no: 1,
          track_of: 1,
          disk_no: 1,
          disk_of: 1,
          is_compilation: false,
          bpm: 120,
          camelot: '4B',
          scan_level: 2,
        },
        {
          id: 't2',
          path: 'D:/Music/01_Afro_House/Adore.mp3',
          title: 'Adore',
          album: 'Single',
          album_artist: 'Hugel',
          artists: ['Hugel'],
          genres: ['Afro House'],
          year: 2025,
          duration: 200,
          track_no: 1,
          track_of: 1,
          disk_no: 1,
          disk_of: 1,
          is_compilation: false,
          bpm: 124,
          camelot: '4B',
          scan_level: 1,
        },
      ],
      [],
    );
    expect(derived).toHaveLength(1);
    expect(derived[0].display_name).toBe('01_Afro_House');
    expect(derived[0].track_count).toBe(2);
    expect(derived[0].full_count).toBe(1);
    expect(derived[0].bpm_min).toBe(120);
    expect(derived[0].bpm_max).toBe(124);
    expect(derived[0].dominant_camelot).toBe('4B');
  });
});
