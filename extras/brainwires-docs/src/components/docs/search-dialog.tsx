"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { CommandDialog, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "@/components/ui/command";
import { FileText, Package, Box } from "lucide-react";
import { NAV_TREE } from "@/lib/nav";

interface SearchEntry { title: string; href: string; group: string; icon: "doc" | "crate" | "extra"; }

function buildIndex(): SearchEntry[] {
  const entries: SearchEntry[] = [];
  for (const item of NAV_TREE) {
    if (item.href && !item.children) entries.push({ title: item.title, href: item.href, group: "Docs", icon: "doc" });
    if (item.children) {
      for (const child of item.children) {
        if (!child.href) continue;
        const isCrate = child.href.startsWith("/crates/");
        const isExtra = child.href.startsWith("/extras/");
        entries.push({ title: child.title, href: child.href, group: item.title, icon: isCrate ? "crate" : isExtra ? "extra" : "doc" });
      }
    }
  }
  return entries;
}

const INDEX = buildIndex();
const GROUPS = Array.from(new Set(INDEX.map((e) => e.group)));

export function SearchDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const router = useRouter();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") { e.preventDefault(); onOpenChange(true); }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onOpenChange]);

  function navigate(href: string) { onOpenChange(false); router.push(href); }

  return (
    <CommandDialog open={open} onOpenChange={onOpenChange}>
      <CommandInput placeholder="Search documentation…" />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>
        {GROUPS.map((group) => (
          <CommandGroup key={group} heading={group}>
            {INDEX.filter((e) => e.group === group).map((entry) => (
              <CommandItem key={entry.href} value={`${entry.title} ${entry.group}`} onSelect={() => navigate(entry.href)} className="gap-2">
                {entry.icon === "crate" ? <Package className="size-4 shrink-0 text-muted-foreground" />
                  : entry.icon === "extra" ? <Box className="size-4 shrink-0 text-muted-foreground" />
                  : <FileText className="size-4 shrink-0 text-muted-foreground" />}
                <span>{entry.title}</span>
                <span className="ml-auto text-xs text-muted-foreground">{entry.group}</span>
              </CommandItem>
            ))}
          </CommandGroup>
        ))}
      </CommandList>
    </CommandDialog>
  );
}
