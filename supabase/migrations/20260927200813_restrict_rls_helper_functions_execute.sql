revoke execute on function public.is_super_admin() from public;
revoke execute on function public.current_tenant_id() from public;

grant execute on function public.is_super_admin() to authenticated;
grant execute on function public.current_tenant_id() to authenticated;
