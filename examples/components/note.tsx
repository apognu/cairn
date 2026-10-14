import { defineComponent, Editable, link, z } from "@cairn/sdk";
import "./note.css";

export default defineComponent({
  name: "acme/note",
  version: 1,
  title: "Acme note",
  description: "A boxed note",
  props: z.object({
    title: z.string().default("Note"),
    body: z.string().default(""),
    page: link(),
  }),
  render: (props) => (
    <aside className="acme-note">
      <strong>
        <Editable path="title" />
      </strong>
      <p>{props.body}</p>
    </aside>
  ),
});
