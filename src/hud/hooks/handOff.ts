export async function handOff(
  onEdit: ((folder: string) => Promise<void>) | undefined,
  folder: string,
): Promise<boolean> {
  if (!onEdit) return true;
  await onEdit(folder);
  return false;
}
