import type { Metadata } from "next";
import { GameConfigEditor } from "@/components/settings/GameConfigEditor";
import { PageHeader } from "@/components/ui/PageHeader";

export const metadata: Metadata = { title: "Settings" };

export default function SettingsPage() {
  return (
    <>
      <PageHeader
        title="Settings"
        status="available"
        description="Game configurations: table size, blinds, effective stack depth and rake. The engine validates every value and rejects inexact or inconsistent input."
      />
      <GameConfigEditor />
    </>
  );
}
