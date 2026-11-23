import type { Metadata } from "next";
import { PreflopWorkbench } from "@/components/preflop/PreflopWorkbench";
import { RangeEditor } from "@/components/poker/RangeEditor";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";
import { StatusBadge } from "@/components/ui/StatusBadge";

export const metadata: Metadata = { title: "Preflop" };

export default function PreflopPage() {
  return (
    <>
      <PageHeader
        title="Preflop"
        status="approximate"
        description="Build and inspect ranges with exact combo counts and card removal, and run GTOpen's preflop model. Preflop results are an approximate model: pots that reach the flop are valued by a model, not solved. Only heads-up push/fold has been validated against an exact solution."
      />
      <div className="grid gap-6">
        <Card>
          <CardHeader title="What the preflop model is" actions={<StatusBadge status="approximate" />} />
          <CardBody className="text-sm text-muted">
            <ul className="list-disc space-y-1.5 pl-5">
              <li>
                GTOpen solves the preflop betting tree for the sizes you allow. When a pot reaches the flop it does not
                play the flop: heads-up pots use all-in equity times a realisation factor, and pots with three or more
                players use a showdown-rank approximation.
              </li>
              <li>Hands are grouped into 169 classes; a blocker model approximates card removal between players.</li>
              <li>
                The &ldquo;model gap&rdquo; measures convergence inside the model. It says nothing about how far the
                model is from real poker.
              </li>
              <li>
                Heads-up push/fold needs no flop model. RiverLab checked it against its own exact combo-level
                equilibrium, so that preset is labelled validated.
              </li>
            </ul>
          </CardBody>
        </Card>
        <PreflopWorkbench />
        <RangeEditor />
      </div>
    </>
  );
}
