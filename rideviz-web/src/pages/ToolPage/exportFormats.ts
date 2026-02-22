import type { ExportPreset } from '../../types/api';

export type ExportFormatOption = {
  value: ExportPreset;
  label: string;
  width: number;
  height: number;
};

export const EXPORT_FORMAT_OPTIONS: ExportFormatOption[] = [
  {
    value: 'story_9x16',
    label: 'Instagram/TikTok Story (1080x1920)',
    width: 1080,
    height: 1920,
  },
  {
    value: 'instagram_post_portrait_4x5',
    label: 'Instagram Post Portrait (1080x1350)',
    width: 1080,
    height: 1350,
  },
  {
    value: 'instagram_post_square_1x1',
    label: 'Instagram Post Square (1080x1080)',
    width: 1080,
    height: 1080,
  },
  {
    value: 'x_post_16x9',
    label: 'X Post Landscape (1600x900)',
    width: 1600,
    height: 900,
  },
  {
    value: 'facebook_feed_landscape',
    label: 'Facebook Feed Landscape (1200x630)',
    width: 1200,
    height: 630,
  },
  {
    value: 'facebook_feed_square',
    label: 'Facebook Feed Square (1080x1080)',
    width: 1080,
    height: 1080,
  },
  {
    value: 'hd_landscape_16x9',
    label: 'HD Landscape (1920x1080)',
    width: 1920,
    height: 1080,
  },
];

export function getExportFormat(value: ExportPreset): ExportFormatOption {
  return (
    EXPORT_FORMAT_OPTIONS.find((format) => format.value === value) ??
    EXPORT_FORMAT_OPTIONS[EXPORT_FORMAT_OPTIONS.length - 1]
  );
}
