/** The head of a block's settings: what the block is, in a sentence, so the fields under it read in context. */
export function PropsHead({ title, description, detail }: { title: string; description?: string; detail?: string }) {
  return (
    <header className="mt-0 mb-4.5 pb-3.5 border-b border-border">
      <h2 className="m-0 text-[17px]">{title}</h2>
      {description && <p className="mt-1 mb-0 text-help leading-[1.45] text-muted">{description}</p>}
      {detail && <span className="block mt-1.5 font-mono text-label text-muted">{detail}</span>}
    </header>
  );
}
