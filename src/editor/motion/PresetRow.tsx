import { Segmented } from "../controls/Controls";

export const CUSTOM = "custom";

export function PresetRow({
  presets,
  value,
  onPick,
}: {
  presets: { id: string; name: string; feel?: string }[];
  value: string;
  onPick: (id: string) => void;
}) {
  const options = presets.map((p) => ({ value: p.id, label: p.name, title: p.feel ?? p.name }));
  if (value === CUSTOM) options.push({ value: CUSTOM, label: "Custom", title: "A curve you drew yourself" });

  return <Segmented value={value} options={options} onChange={onPick} ariaLabel="Motion preset" />;
}
