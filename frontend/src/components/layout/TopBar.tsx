import { Brand } from "./Brand";
import { ConnectionIndicator } from "./ConnectionIndicator";
import { MobileNav } from "./MobileNav";

export function TopBar() {
  return (
    <header className="sticky top-0 z-20 flex h-14 items-center gap-3 border-b border-line bg-canvas/85 px-4 backdrop-blur lg:px-8">
      <MobileNav />
      <div className="lg:hidden">
        <Brand />
      </div>
      <div className="hidden items-center gap-2 text-xs text-muted lg:flex">
        <span className="label-caps">Game</span>
        <span className="text-fg">NL Hold&apos;em · 6-max · Cash</span>
      </div>
      <div className="ml-auto">
        <ConnectionIndicator />
      </div>
    </header>
  );
}
