import { Component, type ReactNode } from "react";

/** Une page qui plante affiche son erreur au lieu de vider toute l'appli. */
export class ErrorBoundary extends Component<{ children: ReactNode; resetKey: string }, { error: Error | null }> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidUpdate(prev: { resetKey: string }) {
    // Changer de page efface l'erreur.
    if (prev.resetKey !== this.props.resetKey && this.state.error) this.setState({ error: null });
  }

  render() {
    if (this.state.error) {
      return (
        <div className="card">
          <p className="danger">
            <strong>Something went wrong on this page.</strong>
          </p>
          <code>{this.state.error.message}</code>
        </div>
      );
    }
    return this.props.children;
  }
}
