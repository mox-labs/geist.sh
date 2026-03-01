// Shared state for the 3D inspect easter egg.
// When true, OrbitControls activate and the user can orbit the scene.

let inspecting = $state(false);

export function toggleInspect() {
	inspecting = !inspecting;
}

export function isInspecting() {
	return inspecting;
}
