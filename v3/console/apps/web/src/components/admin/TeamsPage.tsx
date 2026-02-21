import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { teamsApi } from "@/api/rbac";
import type { Team, TeamFilters } from "@/types/rbac";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { TeamEditor } from "./TeamEditor";
import { Users, Plus, Search, Trash2, Edit, Eye } from "lucide-react";

export function TeamsPage() {
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<TeamFilters>({});
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingTeam, setEditingTeam] = useState<Team | null>(null);
  const [detailTeamId, setDetailTeamId] = useState<string | null>(null);

  const { data, isLoading } = useQuery({
    queryKey: ["admin-teams", filters, page],
    queryFn: () => teamsApi.listTeams(filters, page),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => teamsApi.deleteTeam(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["admin-teams"] }),
  });

  const handleSearch = () => {
    setFilters({ search: search || undefined });
    setPage(1);
  };

  const handleEdit = (team: Team) => {
    setEditingTeam(team);
    setEditorOpen(true);
    setDetailTeamId(null);
  };

  const handleCreate = () => {
    setEditingTeam(null);
    setEditorOpen(true);
    setDetailTeamId(null);
  };

  const handleEditorClose = () => {
    setEditorOpen(false);
    setEditingTeam(null);
  };

  const handleEditorSave = () => {
    queryClient.invalidateQueries({ queryKey: ["admin-teams"] });
    handleEditorClose();
  };

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Users className="h-6 w-6 text-muted-foreground" />
          <div>
            <h1 className="text-2xl font-semibold">Team Workspaces</h1>
            <p className="text-sm text-muted-foreground">
              Organize instances and users into team workspaces
            </p>
          </div>
        </div>
        <Button onClick={handleCreate} className="gap-2">
          <Plus className="h-4 w-4" />
          New Team
        </Button>
      </div>

      {/* Filters */}
      <Card>
        <CardContent className="pt-4">
          <div className="flex gap-2">
            <Input
              placeholder="Search teams..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSearch()}
              className="flex-1 max-w-sm"
            />
            <Button variant="outline" size="icon" onClick={handleSearch}>
              <Search className="h-4 w-4" />
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Teams table */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">
            {data?.pagination.total ?? 0} team{data?.pagination.total !== 1 ? "s" : ""}
          </CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Description</TableHead>
                <TableHead>Members</TableHead>
                <TableHead>Instances</TableHead>
                <TableHead>Created</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableRow>
                  <TableCell colSpan={6} className="text-center py-8 text-muted-foreground">
                    Loading...
                  </TableCell>
                </TableRow>
              ) : data?.teams.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} className="text-center py-8 text-muted-foreground">
                    No teams found
                  </TableCell>
                </TableRow>
              ) : (
                data?.teams.map((team) => (
                  <TableRow key={team.id}>
                    <TableCell>
                      <span className="font-medium text-sm">{team.name}</span>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground max-w-[200px] truncate">
                      {team.description ?? "—"}
                    </TableCell>
                    <TableCell className="text-sm">{team.memberCount}</TableCell>
                    <TableCell className="text-sm">{team.instanceCount}</TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {new Date(team.createdAt).toLocaleDateString()}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex items-center justify-end gap-2">
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={() => setDetailTeamId(detailTeamId === team.id ? null : team.id)}
                          title="View members"
                        >
                          <Eye className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={() => handleEdit(team)}
                        >
                          <Edit className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8 text-destructive hover:text-destructive"
                          onClick={() => {
                            if (
                              confirm(
                                `Delete team "${team.name}"? This will not delete its members or instances.`,
                              )
                            ) {
                              deleteMutation.mutate(team.id);
                            }
                          }}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {/* Team detail panel */}
      {detailTeamId && (
        <TeamDetailPanel teamId={detailTeamId} onClose={() => setDetailTeamId(null)} />
      )}

      {/* Pagination */}
      {data && data.pagination.totalPages > 1 && (
        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <span>
            Page {data.pagination.page} of {data.pagination.totalPages}
          </span>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={page === 1}
              onClick={() => setPage((p) => p - 1)}
            >
              Previous
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={page === data.pagination.totalPages}
              onClick={() => setPage((p) => p + 1)}
            >
              Next
            </Button>
          </div>
        </div>
      )}

      {/* Editor dialog */}
      <TeamEditor
        open={editorOpen}
        team={editingTeam}
        onClose={handleEditorClose}
        onSave={handleEditorSave}
      />
    </div>
  );
}

function TeamDetailPanel({ teamId, onClose }: { teamId: string; onClose: () => void }) {
  const queryClient = useQueryClient();
  const { data: team, isLoading } = useQuery({
    queryKey: ["admin-team-detail", teamId],
    queryFn: () => teamsApi.getTeam(teamId),
  });

  const removeMemberMutation = useMutation({
    mutationFn: (userId: string) => teamsApi.removeMember(teamId, userId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["admin-team-detail", teamId] });
      queryClient.invalidateQueries({ queryKey: ["admin-teams"] });
    },
  });

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base">
            {isLoading ? "Loading..." : `${team?.name} — Members`}
          </CardTitle>
          <Button variant="ghost" size="sm" onClick={onClose}>
            Close
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <p className="text-sm text-muted-foreground">Loading members...</p>
        ) : team?.members.length === 0 ? (
          <p className="text-sm text-muted-foreground">No members yet</p>
        ) : (
          <div className="space-y-2">
            {team?.members.map((member) => (
              <div
                key={member.userId}
                className="flex items-center justify-between py-2 border-b border-border last:border-0"
              >
                <div>
                  <p className="text-sm font-medium">{member.user.name ?? member.user.email}</p>
                  {member.user.name && (
                    <p className="text-xs text-muted-foreground">{member.user.email}</p>
                  )}
                </div>
                <div className="flex items-center gap-3">
                  <span className="text-xs text-muted-foreground">{member.role}</span>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 text-destructive"
                    onClick={() => removeMemberMutation.mutate(member.userId)}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
