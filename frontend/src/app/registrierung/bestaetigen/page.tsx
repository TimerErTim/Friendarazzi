import { Suspense } from "react";
import { EmailVerificationPage } from "@/components/auth/auth-pages";

export default function EmailBestaetigenRoute() {
  return (
    <Suspense fallback={null}>
      <EmailVerificationPage />
    </Suspense>
  );
}
