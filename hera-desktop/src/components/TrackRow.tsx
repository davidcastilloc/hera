import { useSortable } from '@dnd-kit/sortable';
import * as stylex from '@stylexjs/stylex';
import type React from 'react';

import type { Track } from '../generated/typings';
import useFormattedDuration from '../hooks/useFormattedDuration';
import PlayingIndicator from './PlayingIndicator';

export type TrackRowEvents = {
  onTrackSelect: (
    event: React.MouseEvent,
    trackID: string,
    trackIndex: number,
  ) => void;
  onContextMenu: (
    event: React.MouseEvent,
    trackID: string,
    trackIndex: number,
  ) => void;
  onPlaybackStart: (trackID: string) => void;
};

type Props = {
  selected: boolean;
  hasSelectedAbove?: boolean;
  track: Track;
  index: number;
  isPlaying?: boolean;
  draggable?: boolean;
  simplified?: boolean;
  showArtistInTitle?: boolean;
  style?: React.CSSProperties;
} & TrackRowEvents;

const CUE_SLOTS = [
  { num: 1, label: '1:Intro', color: '#22c55e' },
  { num: 2, label: '2:Quiebre', color: '#38bdf8' },
  { num: 3, label: '3:Drop', color: '#ef4444' },
  { num: 4, label: '4:Salida', color: '#f59e0b' },
] as const;

export default function TrackRow(props: Props) {
  const {
    track,
    index,
    selected,
    draggable,
    onTrackSelect,
    onContextMenu,
    onPlaybackStart,
  } = props;

  const duration = useFormattedDuration(track.duration);

  const {
    attributes,
    listeners,
    setNodeRef,
    isDragging,
    isOver,
    activeIndex,
    overIndex,
  } = useSortable({
    id: props.track.id,
    disabled: !draggable,
    data: {
      type: 'playlist-track',
      index,
    },
  });

  const isEvenRow = index % 2 === 0;
  const isSelectedWithSelectedAbove =
    selected && props.hasSelectedAbove === true;
  const isDropIndicatorVisible = isOver && !isDragging;
  const isDropAbove = isOver && overIndex < activeIndex;
  const isDropBelow = isOver && overIndex > activeIndex;

  return (
    // oxlint-disable-next-line jsx_a11y/click-events-have-key-events - given by ...listeners
    <li
      onDoubleClick={() => onPlaybackStart(track.id)}
      onMouseDown={(e) => onTrackSelect(e, track.id, index)}
      onClick={(e) => onTrackSelect(e, track.id, index)}
      onContextMenu={(e) => onContextMenu(e, track.id, index)}
      data-is-playing={props.isPlaying === true}
      data-track-id={track.id}
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      aria-disabled="false"
      aria-selected={selected}
      role="option"
      {...stylex.props(
        trackStyles.track,
        isEvenRow && trackStyles.trackEvenRow,
        selected && trackStyles.trackSelected,
        isSelectedWithSelectedAbove && trackStyles.selectedWithSelectedAbove,
        isDragging && trackStyles.isDragging,
        isDropIndicatorVisible && trackStyles.isDraggedOver,
        isDropAbove && trackStyles.isDraggedOverAbove,
        isDropBelow && trackStyles.isDraggedOverBelow,
      )}
      style={props.style}
    >
      <div {...stylex.props(cellStyles.cell, cellStyles.trackPlaying)}>
        {props.isPlaying ? <PlayingIndicator /> : null}
      </div>
      <div {...stylex.props(cellStyles.cell, cellStyles.title)}>
        <span {...stylex.props(cellStyles.titleText)}>{track.title}</span>
        {props.showArtistInTitle === true && track.artists.length > 0 && (
          <span {...stylex.props(cellStyles.artistInTitle)}>
            {'— '}
            {track.artists.join(', ')}
          </span>
        )}
      </div>
      {props.simplified !== true && (
        <>
          <div {...stylex.props(cellStyles.cell, cellStyles.bpm)}>
            {track.bpm != null ? `${track.bpm.toFixed(1)}` : '—'}
          </div>
          <div {...stylex.props(cellStyles.cell, cellStyles.camelot)}>
            <span
              {...stylex.props(
                cellStyles.camelotBadge,
                selected && cellStyles.camelotBadgeSelected,
              )}
            >
              {track.camelot ?? '8A'}
            </span>
          </div>
          <div
            {...stylex.props(cellStyles.cell, cellStyles.hotCues)}
            title="4 Hot Cues: 1 Intro · 2 Quiebre · 3 Drop · 4 Salida"
          >
            <div {...stylex.props(cellStyles.cueRail)}>
              {CUE_SLOTS.map((cue) => (
                <span key={cue.num} {...stylex.props(cellStyles.cueChip)}>
                  <span
                    {...stylex.props(cellStyles.cueDot)}
                    style={{ backgroundColor: cue.color }}
                  />
                  <span {...stylex.props(cellStyles.cueNum)}>{cue.num}</span>
                </span>
              ))}
            </div>
          </div>
        </>
      )}
      <div
        {...stylex.props(
          cellStyles.cell,
          cellStyles.duration,
          props.simplified === true && cellStyles.rightAligned,
        )}
      >
        {duration}
      </div>
      {props.simplified !== true && (
        <>
          <div {...stylex.props(cellStyles.cell, cellStyles.artist)}>
            {track.artists.join(', ')}
          </div>
          <div {...stylex.props(cellStyles.cell, cellStyles.album)}>
            {track.album}
          </div>
          <div {...stylex.props(cellStyles.cell, cellStyles.genre)}>
            {track.genres.join(', ')}
          </div>
        </>
      )}
    </li>
  );
}

const trackStyles = stylex.create({
  track: {
    position: 'relative',
    display: 'flex',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: 'transparent',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: 'rgba(35, 42, 54, 0.45)',
    backgroundColor: 'var(--tracks-bg-even)',
    alignItems: 'center',
    maxWidth: '100%',
  },
  trackEvenRow: {
    backgroundColor: 'var(--tracks-bg-odd)',
  },
  trackSelected: {
    backgroundColor: 'var(--active-item-bg)',
    color: 'var(--active-item-color)',
  },
  selectedWithSelectedAbove: {
    borderTopColor: 'rgba(45, 212, 191, 0.24)',
  },
  isDragging: {
    opacity: 0.5,
  },
  isDraggedOver: {
    '::after': {
      pointerEvents: 'none',
      position: 'absolute',
      zIndex: 1,
      display: 'block',
      width: '100%',
      content: '""',
      height: '2px',
      backgroundColor: 'var(--main-color)',
    },
  },
  isDraggedOverAbove: {
    '::after': {
      top: '-1px',
    },
  },
  isDraggedOverBelow: {
    '::after': {
      bottom: '-1px',
    },
  },
});

const cellStyles = stylex.create({
  cell: {
    borderLeftWidth: '1px',
    borderLeftStyle: 'solid',
    borderLeftColor: 'transparent',
    paddingRight: '6px',
    paddingLeft: '6px',
    cursor: 'default',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    lineHeight: '24px',
  },
  trackPlaying: {
    width: '30px',
    flexShrink: 0,
  },
  title: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minWidth: 0,
    display: 'flex',
    alignItems: 'baseline',
    rowGap: '4px',
    columnGap: '4px',
  },
  titleText: {
    minWidth: 0,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
    fontWeight: 500,
  },
  artistInTitle: {
    color: 'var(--text-muted)',
    fontSize: '0.85em',
    flexShrink: 1,
    minWidth: 0,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  duration: {
    width: '70px',
    minWidth: '70px',
    flexShrink: 0,
    fontVariantNumeric: 'tabular-nums',
    color: '#8e98a8',
  },
  artist: {
    width: '16%',
    flexShrink: 0,
    color: '#8e98a8',
  },
  album: {
    width: '14%',
    flexShrink: 0,
    color: '#8e98a8',
  },
  genre: {
    width: '12%',
    flexShrink: 0,
    color: '#8e98a8',
  },
  bpm: {
    width: '66px',
    minWidth: '66px',
    flexShrink: 0,
    fontVariantNumeric: 'tabular-nums',
    fontWeight: 600,
    color: '#edeff3',
  },
  camelot: {
    width: '58px',
    minWidth: '58px',
    flexShrink: 0,
    display: 'flex',
    alignItems: 'center',
  },
  camelotBadge: {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    paddingBlock: '1px',
    paddingInline: '6px',
    borderRadius: '4px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    fontSize: '11px',
    fontWeight: 600,
    lineHeight: 1.35,
    fontVariantNumeric: 'tabular-nums',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    color: '#2dd4bf',
  },
  camelotBadgeSelected: {
    backgroundColor: 'rgba(12, 14, 18, 0.55)',
    borderColor: 'rgba(45, 212, 191, 0.45)',
    color: '#2dd4bf',
  },
  hotCues: {
    width: '104px',
    minWidth: '104px',
    flexShrink: 0,
    display: 'flex',
    alignItems: 'center',
  },
  cueRail: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '4px',
    paddingBlock: '2px',
    paddingInline: '5px',
    borderRadius: '4px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    lineHeight: 1,
  },
  cueChip: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '2px',
  },
  cueDot: {
    width: '5px',
    height: '5px',
    borderRadius: '999px',
  },
  cueNum: {
    fontSize: '9.5px',
    fontWeight: 600,
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  rightAligned: {
    textAlign: 'right',
    paddingRight: '8px',
  },
});
