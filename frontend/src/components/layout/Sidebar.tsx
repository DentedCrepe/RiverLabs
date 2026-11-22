import { Brand } from "./Brand";
import { NavList } from "./NavList";
import { StatusLegend } from "./StatusLegend";

export function Sidebar() {
  return (
    <aside className="fixed inset-y-0 left-0 z-30 hidden w-60 flex-col border-r border-line bg-surface lg:flex">
      <div className="flex h-14 items-center border-b border-line px-4">
        <Brand />
      </div>
      <nav aria-label="Main" className="flex-1 overflow-y-auto px-3 py-4">
        <p className="label-caps mb-2 px-2.5">Workspace</p>
        <NavList />
      </nav>
      <div className="border-t border-line py-3">
        <StatusLegend />
      </div>
    </aside>
  );
}
