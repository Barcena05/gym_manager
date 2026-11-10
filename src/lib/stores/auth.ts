import { writable } from 'svelte/store';

interface LoginResponse {
	username: string;
	message: string;
	success: boolean;
	role: string;
}

interface SessionUser {
	username: string;
	role: string;
}

function createAuthStore() {
	const { subscribe, set, update } = writable({
		isAuthenticated: false,
		username: null as string | null,
		error: null as string | null,
		role: null as string | null
	});

	return {
		subscribe,
		restore: async () => {
			const { invoke } = await import('@tauri-apps/api/core');
			try {
				const session = await invoke<SessionUser | null>('get_session');
				if (session) {
					set({
						isAuthenticated: true,
						username: session.username,
						role: session.role,
						error: null
					});
					return true;
				}
			} catch (e) {
				console.error('Session restore error:', e);
			}

			set({ isAuthenticated: false, username: null, role: null, error: null });
			return false;
		},
		login: async (username: string, password: string) => {
			username = username.trim();
			password = password.trim();
			const { invoke } = await import('@tauri-apps/api/core');
			try {
				const response: LoginResponse = await invoke('login', {
					payload: { username, password }
				});

				if (response.success) {
					set({
						isAuthenticated: true,
						username: response.username,
						role: response.role,
						error: null
					});
				} else {
					set({ isAuthenticated: false, username: null, role: null, error: response.message });
					return false;
				}
				return true;
			} catch (e) {
				console.error('Login error:', e);
				set({
					isAuthenticated: false,
					username: null,
					error: e instanceof Error ? e.message : 'An unexpected error occurred.',
					role: null
				});
				return false;
			}
		},
		logout: async () => {
			const { invoke } = await import('@tauri-apps/api/core');
			try {
				await invoke('logout');
			} catch (e) {
				console.error('Logout error:', e);
			}
			set({ isAuthenticated: false, role: null, username: null, error: null });
		},
		clearError: () => {
			update((state) => ({ ...state, error: null }));
		}
	};
}

export const auth = createAuthStore();
