import type { Metadata, Viewport } from "next";
import { Archivo_Black, JetBrains_Mono, Space_Grotesk } from "next/font/google";
import "./globals.css";
import { AppChrome } from "@/components/layout/app-chrome";
import { Providers } from "./providers";

const archivoBlack = Archivo_Black({
  variable: "--font-archivo-black",
  subsets: ["latin"],
  weight: ["400"],
});

const spaceGrotesk = Space_Grotesk({
  variable: "--font-space-grotesk",
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
});

const jetbrainsMono = JetBrains_Mono({
  variable: "--font-jetbrains-mono",
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
});

const basePath = (process.env.NEXT_PUBLIC_FRIENDARAZZI_BASE_PATH ?? "").replace(
  /\/+$/,
  "",
);

const siteUrl =
  process.env.NEXT_PUBLIC_FRIENDARAZZI_SITE_URL ||
  (basePath ? "https://timerertim.github.io" : "https://friendarazzi.app");

const description =
  "Friendarazzi – das soziale Foto-Jagd-Bingo für Freundesgruppen: Lobbys erstellen, witzige Aufträge verteilen, Beweisfotos knipsen und Erinnerungen sammeln.";

const ogImageUrl = `${basePath}/og-image.jpg`;

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#FFE600" },
    { media: "(prefers-color-scheme: dark)", color: "#1C1B18" },
  ],
};

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title: {
    default: "Friendarazzi — Foto-Jagd-Bingo",
    template: "%s · Friendarazzi",
  },
  description,
  applicationName: "Friendarazzi",
  manifest: `${basePath}/site.webmanifest`,
  icons: {
    icon: [
      { url: `${basePath}/favicon.ico`, sizes: "any", type: "image/x-icon" },
      { url: `${basePath}/favicon.svg`, type: "image/svg+xml" },
      {
        url: `${basePath}/favicon-32x32.png`,
        sizes: "32x32",
        type: "image/png",
      },
      {
        url: `${basePath}/favicon-16x16.png`,
        sizes: "16x16",
        type: "image/png",
      },
    ],
    apple: {
      url: `${basePath}/apple-touch-icon.png`,
      sizes: "180x180",
      type: "image/png",
    },
    other: [
      {
        rel: "mask-icon",
        url: `${basePath}/safari-pinned-tab.svg`,
        color: "#0D0D0D",
      },
    ],
  },
  openGraph: {
    type: "website",
    locale: "de_AT",
    siteName: "Friendarazzi",
    title: "Friendarazzi — The Candid Photo-Hunt Bingo",
    description,
    images: [
      {
        url: ogImageUrl,
        width: 1200,
        height: 630,
        alt: "Friendarazzi — The Candid Photo-Hunt Bingo",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "Friendarazzi — The Candid Photo-Hunt Bingo",
    description,
    images: [ogImageUrl],
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="de"
      className={`${archivoBlack.variable} ${spaceGrotesk.variable} ${jetbrainsMono.variable}`}
    >
      <body className="font-sans antialiased">
        <Providers>
          <AppChrome>{children}</AppChrome>
        </Providers>
      </body>
    </html>
  );
}
