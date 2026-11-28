"use client";

import { QueryClientProvider } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { AuthProvider } from "@/api/auth-provider";
import { FriendarazziApiProvider } from "@/api/provider";
import { ToastProvider } from "@/components/feedback/toast-provider";
import { registerPwaSupport } from "@/lib/pwa-install";
import { createQueryClient } from "@/lib/query-client";

export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(() => createQueryClient());

  useEffect(() => {
    registerPwaSupport();
  }, []);

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
