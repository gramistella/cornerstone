<script lang="ts">
	import '../app.css';
	import { tokens, getUserFromToken } from '$lib/auth'; // Changed
	import { postApi } from '$lib/api'; // Import postApi for logout
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	let currentUser: { sub: string } | null = null;
	let showNav = false;

	// Use a reactive statement to update currentUser whenever the access token changes.
	$: currentUser = getUserFromToken($tokens?.accessToken ?? null); // Changed

	async function logout() {
		try {
			await postApi('logout', {}); // Call the new logout endpoint
		} catch (error) {
			console.error('Logout failed, clearing tokens locally anyway.', error);
		} finally {
			tokens.set(null); // This will also remove it from localStorage
			goto('/login');
		}
	}
</script>


<nav class="bg-gray-800 text-white p-4">
	<div class="container mx-auto flex justify-between items-center">
		<a href="/" class="text-xl font-bold">Cornerstone CRM</a>
		<div class="hidden md:flex space-x-4">
			{#if currentUser}
				<a href="/contacts" class:active={$page.url.pathname === '/contacts'}>Contacts</a>
				<span>Welcome, User {currentUser.sub}!</span>
				<button on:click={logout} class="hover:underline">Logout</button>
			{:else}
				<a href="/login" class:active={$page.url.pathname === '/login'}>Login</a>
				<a href="/register" class:active={$page.url.pathname === '/register'}>Register</a>
			{/if}
		</div>
		<div class="md:hidden">
			<button on:click={() => (showNav = !showNav)}>
				<svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16m-7 6h7"></path></svg>
			</button>
		</div>
	</div>
	{#if showNav}
	<div class="md:hidden mt-2">
		{#if currentUser}
			<a href="/contacts" class="block py-2 px-4 hover:bg-gray-700">Contacts</a>
			<span class="block py-2 px-4">Welcome, User {currentUser.sub}!</span>
			<button on:click={logout} class="w-full text-left py-2 px-4 hover:bg-gray-700">Logout</button>
		{:else}
			<a href="/login" class="block py-2 px-4 hover:bg-gray-700">Login</a>
			<a href="/register" class="block py-2 px-4 hover:bg-gray-700">Register</a>
		{/if}
	</div>
	{/if}
</nav>

<main class="container mx-auto p-4">
	<slot />
</main>
