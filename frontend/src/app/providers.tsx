"use client";

import { QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";
import { AuthProvider } from "@/api/auth-provider";
import { FriendarazziApiProvider } from "@/api/provider";
import { ToastProvider } from "@/components/feedback/toast-provider";
import { createQueryClient } from "@/lib/query-client";

export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(() => createQueryClient());

  return (
    <QueryClientProvider client={queryClient}>
      <ToastProvider>
        <FriendarazziApiProvider>
          <AuthProvider>{children}</AuthProvider>
        </FriendarazziApiProvider>
      </ToastProvider>
    </QueryClientProvider>
  );
}
