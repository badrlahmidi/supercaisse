alter policy profiles_update on public.profiles
  using (
    id = auth.uid()
    or public.is_super_admin()
    or (
      is_super_admin = false
      and tenant_id = public.current_tenant_id()
      and public.is_admin_tenant()
    )
  )
  with check (
    (id = auth.uid() and is_super_admin = false)
    or public.is_super_admin()
    or (
      is_super_admin = false
      and tenant_id = public.current_tenant_id()
      and public.is_admin_tenant()
    )
  );
