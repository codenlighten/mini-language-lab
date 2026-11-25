export interface Example {
  id: string;
  title: string;
  source: string;
}

export const EXAMPLES: Example[] = [
  {
    id: "order",
    title: "Order handling (valid)",
    source: `// Order handling
process Order {
  start -> check
  task check "Check stock" -> decide
  gateway decide "In stock?" {
    yes -> ship,
    no -> backorder
  }
  task ship "Ship the order" -> done
  task backorder "Order from supplier" -> check
  end done "Order complete"
}
`,
  },
  {
    id: "approval",
    title: "Approval with a retry loop (valid)",
    source: `process Approval {
  start -> draft
  task draft "Write the request" -> review
  gateway review "Approved?" {
    approved -> publish,
    changes -> draft,
    rejected -> closed
  }
  task publish "Publish" -> done
  end done "Published"
  end closed "Closed without publishing"
}
`,
  },
  {
    id: "mistakes",
    title: "A tour of the mistakes it catches",
    source: `// Every process below has something to find.
// Each process shows one kind of mistake: wrong structure, missing pieces, a mistyped name, or invalid text.
process Broken {
  start -> intake
  task intake "Take the order" -> pick
  gateway pick "Which way?" {
    fast -> ship,
    slow -> loop1
  }
  task ship "Ship it" -> done
  task ship "Ship twice" -> done            // duplicate name
  task review "Orphan step" -> done         // nothing leads here
  task loop1 -> loop2
  task loop2 -> loop1                       // reachable, but can never finish
  end done
}

process Incomplete {
  start -> pick
  gateway pick "One way only" {             // a gateway needs two branches
    only -> wait
  }
  task wait "Wait"                          // a task must say what comes next
  end done
}

process Typo {
  start -> check
  task check "Check" -> sihp                // typo: did you mean ship?
  task ship "Ship it" -> done
  end done
}

process Syntax {
  start -> a
  task a "Missing arrow" b
  gateway g { yes -> a no -> a }
  # stray text is reported once
  end e
}
`,
  },
  {
    id: "unicode",
    title: "Unicode in comments and labels",
    source: `// Pedidos 📦 — comments and labels may contain any text
process Pedido {
  start -> revisar
  task revisar "Revisar el stock ✓" -> enviar
  task enviar "Enviar 🚚" -> listo
  end listo "Listo 🎉"
}
`,
  },
];
