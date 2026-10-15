/** The head of a block's settings: what the block is, in a sentence, so the fields under it read in context. */
export function PropsHead({ title, description, detail }: { title: string; description?: string; detail?: string }) {
  return (
    <header className="props-head">
      <h2>{title}</h2>
      {description && <p className="muted">{description}</p>}
      {detail && <span className="mono muted">{detail}</span>}
    </header>
  );
}
