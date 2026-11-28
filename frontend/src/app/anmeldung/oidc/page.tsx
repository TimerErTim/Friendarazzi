import { Suspense } from "react";
import { OidcCallbackPage } from "@/components/auth/auth-pages";

export default function OidcCallbackRoute() {
  return (
    <Suspense fallback={null}>
      <OidcCallbackPage />
    </Suspense>
  );
}
