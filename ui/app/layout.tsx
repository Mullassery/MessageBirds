import type { ReactNode } from "react";
import "./globals.css";

export const metadata = {
  title: "MessageBirds",
  description: "Profile timeline viewer",
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <header>
          <h1>
            <a href="/">MessageBirds</a>
          </h1>
          <p className="muted">
            Profile lookup · <a href="/audiences">Audiences</a> ·{" "}
            <a href="/data-quality">Data quality</a> · <a href="/governance">Governance</a> ·{" "}
            <a href="/policy-simulator">Policy simulator</a> ·{" "}
            <a href="/channels">Channels</a> · <a href="/templates">Templates</a> ·{" "}
            <a href="/journeys">Journeys</a>
          </p>
        </header>
        {children}
      </body>
    </html>
  );
}
